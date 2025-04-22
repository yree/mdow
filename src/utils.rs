use axum::response::IntoResponse;
use sqlx::SqlitePool;
use pulldown_cmark::{html::push_html, Options, Parser};
use qrcode::{render::svg, QrCode};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use axum::response::Html;

pub fn clean(content: &str) -> String {
    ammonia::clean(content)
}

pub fn convert_markdown_to_html(markdown_content: &str) -> String {
    let markdown_options = set_markdown_parser_options();
    let parser = Parser::new_ext(markdown_content, markdown_options);
    let mut html_output = String::new();
    push_html(&mut html_output, parser);

    add_syntax_highlighting_containers(html_output)
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
    Uuid::new_v4().to_string()[..7].to_string()
}

pub fn generate_qr_svg(id: &str) -> String {
    let url = format!("https://mdow.yree.io/view/{}", id);
    let code = QrCode::new(url).expect("Failed to generate QR code");
    let svg = code.render::<svg::Color>().min_dimensions(64, 64).build();
    svg
}

pub async fn save_markdown_document(
    pool: &SqlitePool,
    id: &str,
    content: &str,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) {
    sqlx::query(
        r#"
        INSERT INTO markdown_documents (id, content, created_at, expires_at)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(id)
    .bind(content)
    .bind(created_at)
    .bind(expires_at)
    .execute(pool)
    .await
    .expect("Failed to save document");
}

pub fn handle_404() -> Html<String> {
    Html(
        crate::views::create_404_page().into_string(),
    )
}
