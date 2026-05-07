mod database;
mod handlers;
mod utils;
mod models;
mod views;

use crate::database::setup_database;
use crate::handlers::{handle_main_request, handle_preview_request, handle_share_request, handle_view_request, handle_unlock_request, handle_debug_request};
use crate::utils::{handle_404, RateLimiter};
use axum::{
    http::StatusCode,
    routing::{get, post},
    Extension, Router,
};
use sqlx::sqlite::SqlitePool;
use std::net::SocketAddr;
use std::sync::Arc;

const DEFAULT_PORT: u16 = 8081;
const DEFAULT_DB_PATH: &str = "sqlite:data/database.db";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<()> {
    let pool = setup_database().await?;
    let app = setup_router(pool);
    let addr = get_server_addr();
    println!("Listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    Ok(())
}

fn setup_router(pool: SqlitePool) -> Router {
    let rate_limiter = Arc::new(RateLimiter::new());
    Router::new()
        .route("/", get(handle_main_request))
        .route("/preview", post(handle_preview_request))
        .route("/share", post(handle_share_request))
        .route("/view/:id", get(handle_view_request))
        .route("/unlock/:id", post(handle_unlock_request))
        .route("/debug", get(handle_debug_request))
        .fallback(|| async { (StatusCode::NOT_FOUND, handle_404()) })
        .layer(Extension(rate_limiter))
        .with_state(pool)
}

fn get_server_addr() -> SocketAddr {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    SocketAddr::from(([0, 0, 0, 0], port))
}
