use sqlx::SqlitePool;
use pulldown_cmark::{html::push_html, Options, Parser};
use qrcode::{render::svg, QrCode};
use uuid::Uuid;
use axum::response::{Html, IntoResponse};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use rand_core::OsRng;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use crate::Result;

const MAX_ATTEMPTS: u32 = 5;
const LOCKOUT_DURATION: Duration = Duration::from_secs(10 * 60);

pub struct RateLimiter(Mutex<HashMap<(IpAddr, String), (u32, Instant)>>);

impl RateLimiter {
    pub fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }

    pub fn secs_remaining(&self, ip: IpAddr, doc_id: &str) -> Option<u64> {
        let map = self.0.lock().unwrap();
        match map.get(&(ip, doc_id.to_owned())) {
            Some((count, since)) if *count >= MAX_ATTEMPTS && since.elapsed() < LOCKOUT_DURATION => {
                Some((LOCKOUT_DURATION - since.elapsed()).as_secs().max(1))
            }
            _ => None,
        }
    }

    pub fn record_failure(&self, ip: IpAddr, doc_id: &str) {
        let mut map = self.0.lock().unwrap();
        let entry = map.entry((ip, doc_id.to_owned())).or_insert((0, Instant::now()));
        if entry.1.elapsed() >= LOCKOUT_DURATION {
            *entry = (1, Instant::now());
        } else {
            entry.0 += 1;
        }
    }

    pub fn reset(&self, ip: IpAddr, doc_id: &str) {
        self.0.lock().unwrap().remove(&(ip, doc_id.to_owned()));
    }
}

fn clean(content: &str) -> String {
    ammonia::clean(content)
}

pub fn convert_markdown_to_html(markdown_content: &str) -> String {
    let markdown_options = set_markdown_parser_options();
    let parser = Parser::new_ext(markdown_content, markdown_options);
    let mut html_output = String::new();
    push_html(&mut html_output, parser);
    let html_output = add_syntax_highlighting_containers(html_output);
    clean(&html_output)
}

fn set_markdown_parser_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options
}

fn add_syntax_highlighting_containers(html: String) -> String {
    html.replace("<pre>", "<div class=\"highlighter-rouge\"><pre>")
        .replace("</pre>", "</pre></div>")
}

pub fn extract_title_from_html(html_content: &str) -> Option<&str> {
    html_content.find("<h1>").and_then(|start| {
        html_content[start..]
            .find("</h1>")
            .map(|end| &html_content[start + 4..start + end])
    })
}

pub fn create_htmx_redirect_response(document_id: &str) -> impl IntoResponse {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "hx-redirect",
        format!("/view/{}", document_id).parse().unwrap(),
    );
    (headers, "")
}

pub fn generate_short_uuid() -> String {
    Uuid::new_v4().to_string()[..8].to_string()
}

pub fn generate_qr_svg(id: &str) -> String {
    let url = format!("https://mdow.yree.io/view/{}", id);
    let code = QrCode::new(url).expect("Failed to generate QR code");
    code.render::<svg::Color>().min_dimensions(64, 64).build()
}

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string().into())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|h| Argon2::default().verify_password(password.as_bytes(), &h).is_ok())
        .unwrap_or(false)
}

pub async fn save_document(pool: &SqlitePool, id: &str, content: &str, days: i64, password: Option<String>) -> Result<()> {
    sqlx::query("INSERT INTO documents (id, content, days, password) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(content)
        .bind(days)
        .bind(password)
        .execute(pool)
        .await?;
    Ok(())
}

pub fn handle_404() -> Html<String> {
    Html(
        crate::views::create_404_page().into_string(),
    )
}
