use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};
use tokio::fs;

pub async fn auth_page_handler() -> impl IntoResponse {
    match fs::read_to_string("auth.html").await {
        Ok(html) => (StatusCode::OK, Html(html)).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("auth.html not found or unreadable: {}", err),
        )
            .into_response(),
    }
}
