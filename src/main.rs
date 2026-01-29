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

struct AppState {
    allowed_ips: RwLock<HashSet<String>>,
    target_port: u16,
}

#[tokio::main]
async fn main() {
    // 1. 获取并校验参数
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 {
        println!("用法: ./proxy <API_PORT> <TOKEN> <IN_PORT> <OUT_PORT>");
        println!("示例: ./proxy 28901 MySecretKey 21180 22180");
        return;
    }

    let api_port: u16 = args[1].parse().expect("API 端口无效");
    let token = args[2].clone();
    let in_port: u16 = args[3].parse().expect("监听端口无效");
    let out_port: u16 = args[4].parse().expect("目标端口无效");

    let state = Arc::new(AppState {
        allowed_ips: RwLock::new(HashSet::new()),
        target_port: out_port,
    });

    // 2. 启动 HTTP API 控制平面
    let http_state = Arc::clone(&state);
    let app = Router::new()
        .route(&format!("/{}", token), get(add_ip_handler))
        .route("/list", get(list_ips_handler))
        .with_state(http_state);

    let api_listener = TcpListener::bind(format!("0.0.0.0:{}", api_port))
        .await
        .expect("无法绑定 API 端口");

    println!("========================================");
    println!("🚀 服务已启动");
    println!("🔑 API 地址: http://0.0.0.0:{}/{}", api_port, token);
    println!("📋 列表地址: http://0.0.0.0:{}/list", api_port);
    println!("🛡️  转发路径: :{} -> 127.0.0.1:{}", in_port, out_port);
    println!("========================================");

    tokio::spawn(async move {
        axum::serve(
            api_listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    // 3. 启动 TCP 转发逻辑
    let proxy_listener = TcpListener::bind(format!("0.0.0.0:{}", in_port))
        .await
        .expect("无法绑定监听端口");

    loop {
        let (mut inbound, peer_addr) = match proxy_listener.accept().await {
            Ok(res) => res,
            Err(_) => continue,
        };

        let client_ip = peer_addr.ip().to_string();
        let state_ref = Arc::clone(&state);

        tokio::spawn(async move {
            let is_allowed = {
                let ips = state_ref.allowed_ips.read().unwrap();
                ips.contains(&client_ip)
            };
            
            if !is_allowed {
                // 非白名单 IP 尝试连接时，直接静默关闭
                return;
            }

            // 连接本地目标服务
            if let Ok(mut outbound) = TcpStream::connect(format!("127.0.0.1:{}", state_ref.target_port)).await {
                // 双向透传流量
                let _ = copy_bidirectional(&mut inbound, &mut outbound).await;
            }
        });
    }
}

// --- 控制器函数 ---

async fn add_ip_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    state: State<Arc<AppState>>,
) -> String {
    let ip = addr.ip().to_string();
    let mut ips = state.allowed_ips.write().unwrap();
    ips.insert(ip.clone());
    format!("SUCCESS: IP {} 已加入白名单", ip)
}

async fn list_ips_handler(state: State<Arc<AppState>>) -> Json<Vec<String>> {
    let ips = state.allowed_ips.read().unwrap();
    Json(ips.iter().cloned().collect())
}