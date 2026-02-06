use axum::{
    extract::{ConnectInfo, State},
    routing::get,
    Json, Router,
};
use std::collections::HashSet;
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

const TCP_IDLE_TIMEOUT: Duration = Duration::from_secs(300); // 5 min

struct AppState {
    allowed_ips: RwLock<HashSet<String>>,
    dest_addr: String,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("用法: ./tcp-auth-proxy <http_port>-<authkey>-<listen_port>-<dest>");
        return;
    }

    let parts: Vec<&str> = args[1].split('-').collect();
    if parts.len() < 4 {
        println!("参数格式错误");
        return;
    }

    let http_port: u16 = parts[0].parse().unwrap();
    let auth_key = parts[1].to_string();
    let listen_port: u16 = parts[2].parse().unwrap();

    let dest_addr = if parts[3].contains(':') {
        parts[3].to_string()
    } else {
        let port: u16 = parts[3].parse().unwrap();
        format!("127.0.0.1:{}", port)
    };

    let state = Arc::new(AppState {
        allowed_ips: RwLock::new(HashSet::new()),
        dest_addr: dest_addr.clone(),
    });

    // HTTP 管理服务
    let app = Router::new()
        .route(&format!("/{}", auth_key), get(add_ip_handler))
        .route("/list", get(list_ips_handler))
        .with_state(Arc::clone(&state));

    let http_listener = TcpListener::bind(format!("0.0.0.0:{}", http_port))
        .await
        .unwrap();

    println!("========================================");
    println!("🚀 TCP 授权代理启动");
    println!("🔗 管理接口: http://<IP>:{}/{}", http_port, auth_key);
    println!("🛡️  转发: :{} -> {}", listen_port, dest_addr);
    println!("⏱️  TCP idle timeout: {} 秒", TCP_IDLE_TIMEOUT.as_secs());
    println!("========================================");

    tokio::spawn(async move {
        axum::serve(
            http_listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    // TCP Proxy
    let proxy_listener = TcpListener::bind(format!("0.0.0.0:{}", listen_port))
        .await
        .unwrap();

    loop {
        let (mut inbound, peer_addr) = match proxy_listener.accept().await {
            Ok(v) => v,
            Err(_) => continue,
        };

        let client_ip = peer_addr.ip().to_string();
        let state_ref = Arc::clone(&state);

        tokio::spawn(async move {
            let allowed = {
                let ips = state_ref.allowed_ips.read().unwrap();
                ips.contains(&client_ip)
            };

            if !allowed {
                return;
            }

            let Ok(mut outbound) = TcpStream::connect(&state_ref.dest_addr).await else {
                return;
            };

            // ⭐ TCP idle timeout 核心逻辑
            let _ = timeout(
                TCP_IDLE_TIMEOUT,
                copy_bidirectional(&mut inbound, &mut outbound),
            )
            .await;
        });
    }
}

// ---------------- handlers ----------------

async fn add_ip_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    state: State<Arc<AppState>>,
) -> String {
    let ip = addr.ip().to_string();
    state.allowed_ips.write().unwrap().insert(ip.clone());
    format!("OK: IP {} 已授权", ip)
}

async fn list_ips_handler(
    state: State<Arc<AppState>>,
) -> Json<Vec<String>> {
    let ips = state.allowed_ips.read().unwrap();
    Json(ips.iter().cloned().collect())
}

