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
    dest_port: u16,
}

#[tokio::main]
async fn main() {
    // 1. 获取并解析复合参数
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("用法: ./proxy <http_port>-<authkey>-<listen_port>-<dest_port>");
        println!("示例: ./proxy 28901-UP8TR7iWp-22180-21180");
        return;
    }

    let raw_config = &args[1];
    let parts: Vec<&str> = raw_config.split('-').collect();
    
    if parts.len() != 4 {
        println!("错误: 参数格式不正确，应为 4 个部分（用 '-' 分割）");
        return;
    }

    let http_port: u16 = parts[0].parse().expect("无效的 http-manage-port");
    let auth_key = parts[1].to_string();
    let listen_port: u16 = parts[2].parse().expect("无效的 listenport");
    let dest_port: u16 = parts[3].parse().expect("无效的 destport");

    let state = Arc::new(AppState {
        allowed_ips: RwLock::new(HashSet::new()),
        dest_port,
    });

    // 2. 启动 HTTP 管理服务
    let http_state = Arc::clone(&state);
    let app = Router::new()
        .route(&format!("/{}", auth_key), get(add_ip_handler))
        .route("/list", get(list_ips_handler))
        .with_state(http_state);

    let http_addr = format!("0.0.0.0:{}", http_port);
    let http_listener = TcpListener::bind(&http_addr).await.expect("无法绑定管理端口");

    println!("========================================");
    println!("🚀 代理服务已启动");
    println!("🔗 管理地址: http://<IP>:{}/{}", http_port, auth_key);
    println!("🛡️  转发配置: :{} -> 127.0.0.1:{}", listen_port, dest_port);
    println!("========================================");

    tokio::spawn(async move {
        axum::serve(
            http_listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    // 3. 启动 TCP 转发服务
    let proxy_listener = TcpListener::bind(format!("0.0.0.0:{}", listen_port))
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
            // 检查白名单
            let is_allowed = {
                let ips = state_ref.allowed_ips.read().unwrap();
                ips.contains(&client_ip)
            };

            if !is_allowed {
                return; // 拒绝非法连接
            }

            // 转发到目标端口
            if let Ok(mut outbound) = TcpStream::connect(format!("127.0.0.1:{}", state_ref.dest_port)).await {
                let _ = copy_bidirectional(&mut inbound, &mut outbound).await;
            }
        });
    }
}

// --- 处理函数 ---

async fn add_ip_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    state: State<Arc<AppState>>,
) -> String {
    let ip = addr.ip().to_string();
    let mut ips = state.allowed_ips.write().unwrap();
    ips.insert(ip.clone());
    format!("OK: IP {} 已授权", ip)
}

async fn list_ips_handler(state: State<Arc<AppState>>) -> Json<Vec<String>> {
    let ips = state.allowed_ips.read().unwrap();
    Json(ips.iter().cloned().collect())
}