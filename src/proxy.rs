use crate::auth::{AuthState, build_auth_key, start_auth_service};
use crate::config::{AppConfig, ProxyRule};
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, copy_bidirectional};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

const TCP_IDLE_TIMEOUT: Duration = Duration::from_secs(2 * 24 * 3600);

pub async fn start_proxy(config: AppConfig) -> Result<(), String> {
    let auth_key = resolve_auth_key(&config)?;
    let management_base_url = build_management_base_url(config.run_on_host.as_deref(), config.http_port);

    if config.run_as_client {
        run_as_client(&config, &auth_key).await?;
        return Ok(());
    }

    let state = Arc::new(AuthState::new());
    start_auth_service(
        config.http_port,
        &auth_key,
        Arc::clone(&state),
    )
    .await?;

    println!("========================================");
    println!("🚀 TCP 授权代理启动");
    if config.auth_key.is_none() {
        println!("🔑 自动生成 auth-key: {}", auth_key);
    }
    println!(
        "🔐 管理接口: {}/<hex_lower(sha256(client_ip + auth_key))>",
        management_base_url
    );
    println!(
        "💻 客户端命令: {}",
        build_client_command(config.run_on_host.as_deref(), config.http_port, &auth_key)
    );
    println!("⏱️  TCP idle timeout: {} 秒", TCP_IDLE_TIMEOUT.as_secs());
    for rule in &config.proxy_rules {
        println!("🛡️  转发: :{} -> {}", rule.listen_port, rule.dest_addr);
    }
    println!("========================================");

    for rule in config.proxy_rules {
        start_proxy_listener(rule, Arc::clone(&state), auth_key.clone()).await?;
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

async fn run_as_client(config: &AppConfig, auth_key: &str) -> Result<(), String> {
    let authority = build_management_authority(config.run_on_host.as_deref(), config.http_port)?;
    let client_ip = http_get_text(&authority, "/ip").await?;
    let key = build_auth_key(&client_ip, auth_key);
    let response = http_get_text(&authority, &format!("/{}", key)).await?;
    println!("{}", response);
    Ok(())
}

fn resolve_auth_key(config: &AppConfig) -> Result<String, String> {
    match &config.auth_key {
        Some(auth_key) => Ok(auth_key.clone()),
        None => generate_auth_key(),
    }
}

fn generate_auth_key() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(|err| format!("自动生成 auth-key 失败: {}", err))?;
    Ok(bytes.iter().map(|byte| format!("{:02x}", byte)).collect())
}

fn build_client_command(run_on_host: Option<&str>, http_port: u16, auth_key: &str) -> String {
    let host = run_on_host.expect("run_on_host must be validated before start_proxy");
    format!(
        "./tcp-auth-proxy --http-port={} --auth-key={} --run-as-client=true --run-on-host={}",
        http_port,
        shell_escape(auth_key),
        shell_escape(host)
    )
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn build_management_authority(run_on_host: Option<&str>, http_port: u16) -> Result<String, String> {
    match run_on_host {
        Some(host) if host.contains(':') => Ok(host.to_string()),
        Some(host) => Ok(format!("{}:{}", host, http_port)),
        None => Err("缺少参数: --run-on-host=<servername>".to_string()),
    }
}

async fn http_get_text(authority: &str, path: &str) -> Result<String, String> {
    let mut stream = TcpStream::connect(authority)
        .await
        .map_err(|err| format!("连接 {} 失败: {}", authority, err))?;

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path, authority
    );
    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|err| format!("请求 {}{} 失败: {}", authority, path, err))?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .map_err(|err| format!("读取 {}{} 响应失败: {}", authority, path, err))?;

    let response = String::from_utf8(response).map_err(|err| format!("响应不是合法 UTF-8: {}", err))?;
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| format!("{}{} 返回了非法 HTTP 响应", authority, path))?;
    let status_line = headers.lines().next().unwrap_or_default();
    if !status_line.contains(" 200 ") {
        return Err(format!("请求 {}{} 失败: {}", authority, path, status_line));
    }

    Ok(body.trim().to_string())
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
                println!("FORBIDDEN: {}", client_ip);
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
