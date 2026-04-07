use axum::{
    extract::{ConnectInfo, State},
    routing::get,
    Json, Router,
};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;

pub struct AuthState {
    allowed_ips: RwLock<HashSet<String>>,
}

impl AuthState {
    pub fn new() -> Self {
        Self {
            allowed_ips: RwLock::new(HashSet::new()),
        }
    }

    pub fn is_allowed(&self, client_ip: &str, auth_key: &str) -> bool {
        let ips = self.allowed_ips.read().unwrap();
        ips.contains(client_ip) || auth_key == "noauthkey"
    }
}

pub async fn start_auth_service(
    http_port: u16,
    auth_key: &str,
    state: Arc<AuthState>,
) -> Result<(), String> {
    let app = Router::new()
        .route(&format!("/{}", auth_key), get(add_ip_handler))
        .route("/list", get(list_ips_handler))
        .with_state(state);

    let http_listener = TcpListener::bind(format!("0.0.0.0:{}", http_port))
        .await
        .map_err(|err| format!("HTTP 端口 {} 绑定失败: {}", http_port, err))?;

    tokio::spawn(async move {
        axum::serve(
            http_listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    Ok(())
}

async fn add_ip_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    state: State<Arc<AuthState>>,
) -> String {
    let ip = addr.ip().to_string();
    state.allowed_ips.write().unwrap().insert(ip.clone());
    format!("OK: IP {} 已授权", ip)
}

async fn list_ips_handler(state: State<Arc<AuthState>>) -> Json<Vec<String>> {
    let ips = state.allowed_ips.read().unwrap();
    Json(ips.iter().cloned().collect())
}
