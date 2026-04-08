use crate::auth_page::auth_page_handler;
use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::process::Command;
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
        .route("/", get(auth_page_handler))
        .route("/ip", get(client_ip_handler))
        .route("/list", get(list_ips_handler))
        .route("/:provided_key", get(add_ip_handler))
        .with_state(Arc::new(AuthServiceState {
            auth_state: state,
            auth_key: auth_key.to_string(),
        }))
        ;

    let http_listener = TcpListener::bind(format!("0.0.0.0:{}", http_port))
        .await
        .map_err(|err| format!("Failed to bind HTTP port {}: {}", http_port, err))?;

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
    Path(provided_key): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    state: State<Arc<AuthServiceState>>,
) -> (StatusCode, String) {
    let ip = addr.ip().to_string();
    if !state.is_valid_key(&ip, &provided_key) {
        println!("[{}] FORBIDDEN: {}", current_beijing_time(), ip);
        return (StatusCode::FORBIDDEN, "FORBIDDEN".to_string());
    }

    let inserted = state.auth_state.allowed_ips.write().unwrap().insert(ip.clone());
    if inserted {
        println!("[{}] Authorized IP added: {}", current_beijing_time(), ip);
    }
    (StatusCode::OK, format!("OK: IP {} is authorized", ip))
}

async fn list_ips_handler(state: State<Arc<AuthServiceState>>) -> Json<Vec<String>> {
    let ips = state.auth_state.allowed_ips.read().unwrap();
    Json(ips.iter().cloned().collect())
}

async fn client_ip_handler(ConnectInfo(addr): ConnectInfo<SocketAddr>) -> String {
    addr.ip().to_string()
}

fn current_beijing_time() -> String {
    Command::new("date")
        .env("TZ", "Asia/Shanghai")
        .arg("+%Y-%m-%d %H:%M:%S")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown-time".to_string())
}

struct AuthServiceState {
    auth_state: Arc<AuthState>,
    auth_key: String,
}

impl AuthServiceState {
    fn is_valid_key(&self, client_ip: &str, provided_key: &str) -> bool {
        provided_key == build_auth_key(client_ip, &self.auth_key)
    }
}

pub fn build_auth_key(client_ip: &str, auth_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(client_ip.as_bytes());
    hasher.update(auth_key.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::build_auth_key;

    #[test]
    fn returns_lowercase_hex_sha256() {
        assert_eq!(
            build_auth_key("222.210.167.214", "987654321"),
            "0b19e59c37c7c9a1000c8d037b2e6237a2703845bad3df8ff1efb232078e7b99"
        );
    }
}
