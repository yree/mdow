mod database;
mod handlers;
mod utils;
mod models;
mod views;

use crate::database::setup_database;
use crate::handlers::{handle_main_request, handle_preview_request, handle_edit_request, handle_share_request, handle_view_request};
use crate::models::{MarkdownInput, MarkdownDocument, RenderParams};
use crate::utils::{save_markdown_document, generate_short_uuid, create_htmx_redirect_response, clean, convert_markdown_to_html, handle_404};
use crate::views::{create_markdown_editor_page, create_markdown_viewer_page};
use axum::{
    http::StatusCode,
    routing::{get, post},
    Router,
};
use sqlx::sqlite::SqlitePool;
use std::net::SocketAddr;

const DEFAULT_PORT: u16 = 8081;
// const DEFAULT_DB_PATH: &str = "sqlite:data/database.db";
const DEFAULT_DB_PATH: &str = "test.db";
const DOCUMENT_EXPIRY_DAYS: i64 = 30;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    let pool = setup_database().await?;
    let app = setup_router(pool);
    let addr = get_server_addr();
    println!("Listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

fn setup_router(pool: SqlitePool) -> Router {
    Router::new()
        .route("/", get(handle_main_request))
        .route("/preview", post(handle_preview_request))
        .route("/edit", post(handle_edit_request))
        .route("/share", post(handle_share_request))
        .route("/view/:id", get(handle_view_request))
        .fallback(|| async { (StatusCode::NOT_FOUND, handle_404()) })
        .with_state(pool)
}

fn get_server_addr() -> SocketAddr {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    SocketAddr::from(([0, 0, 0, 0], port))
}

