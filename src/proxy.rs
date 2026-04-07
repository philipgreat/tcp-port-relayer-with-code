use crate::auth::{AuthState, build_auth_key, start_auth_service};
use crate::config::{AppConfig, ProxyRule};
use std::sync::Arc;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

const TCP_IDLE_TIMEOUT: Duration = Duration::from_secs(2 * 24 * 3600);

pub async fn start_proxy(config: AppConfig) -> Result<(), String> {
    let management_base_url = build_management_base_url(config.run_on_host.as_deref(), config.http_port);

    if let Some(mock_ip) = &config.mock_ip {
        let auth_key = build_auth_key(mock_ip, &config.auth_key, config.enable_hash);
        println!("{}/{}", management_base_url, auth_key);
        return Ok(());
    }

    let state = Arc::new(AuthState::new());
    start_auth_service(
        config.http_port,
        &config.auth_key,
        config.enable_hash,
        Arc::clone(&state),
    )
    .await?;

    println!("========================================");
    println!("🚀 TCP 授权代理启动");
    if config.enable_hash {
        println!(
            "🔐 管理接口: {}/<base64(sha256(client_ip + auth_key))>",
            management_base_url
        );
    } else {
        println!("🔗 管理接口: {}/{}", management_base_url, config.auth_key);
    }
    println!("⏱️  TCP idle timeout: {} 秒", TCP_IDLE_TIMEOUT.as_secs());
    for rule in &config.proxy_rules {
        println!("🛡️  转发: :{} -> {}", rule.listen_port, rule.dest_addr);
    }
    println!("========================================");

    for rule in config.proxy_rules {
        start_proxy_listener(rule, Arc::clone(&state), config.auth_key.clone()).await?;
    }

    Ok(())
}

fn build_management_base_url(run_on_host: Option<&str>, http_port: u16) -> String {
    match run_on_host {
        Some(host) if host.contains(':') => format!("http://{}", host),
        Some(host) => format!("http://{}:{}", host, http_port),
        None => format!("http://<IP>:{}", http_port),
    }
}

async fn start_proxy_listener(
    rule: ProxyRule,
    state: Arc<AuthState>,
    auth_key: String,
) -> Result<(), String> {
    let proxy_listener = TcpListener::bind(format!("0.0.0.0:{}", rule.listen_port))
        .await
        .map_err(|err| format!("TCP 端口 {} 绑定失败: {}", rule.listen_port, err))?;

    tokio::spawn(run_proxy_listener(
        proxy_listener,
        state,
        auth_key,
        rule.dest_addr,
    ));

    Ok(())
}

async fn run_proxy_listener(
    proxy_listener: TcpListener,
    state: Arc<AuthState>,
    auth_key: String,
    dest_addr: String,
) {
    loop {
        let (mut inbound, peer_addr) = match proxy_listener.accept().await {
            Ok(v) => v,
            Err(_) => continue,
        };

        let client_ip = peer_addr.ip().to_string();
        let state_ref = Arc::clone(&state);
        let key = auth_key.clone();
        let target_addr = dest_addr.clone();
        tokio::spawn(async move {
            let allowed = state_ref.is_allowed(&client_ip, &key);

            if !allowed {
                return;
            }

            let Ok(mut outbound) = TcpStream::connect(&target_addr).await else {
                return;
            };

            let _ = timeout(
                TCP_IDLE_TIMEOUT,
                copy_bidirectional(&mut inbound, &mut outbound),
            )
            .await;
        });
    }
}
