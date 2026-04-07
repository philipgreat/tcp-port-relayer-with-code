use crate::auth::{start_auth_service, AuthState};
use crate::config::{AppConfig, ProxyRule};
use std::sync::Arc;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

const TCP_IDLE_TIMEOUT: Duration = Duration::from_secs(2 * 24 * 3600);

pub async fn start_proxy(config: AppConfig) -> Result<(), String> {
    let state = Arc::new(AuthState::new());
    start_auth_service(config.http_port, &config.auth_key, Arc::clone(&state)).await?;

    println!("========================================");
    println!("🚀 TCP 授权代理启动");
    println!("🔗 管理接口: http://<IP>:{}/{}", config.http_port, config.auth_key);
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
