use sqlx::SqlitePool;
use pulldown_cmark::{html::push_html, Options, Parser};
use qrcode::{render::svg, QrCode};
use uuid::Uuid;
use axum::response::{Html, IntoResponse};
use crate::Result;

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

pub async fn save_document(pool: &SqlitePool, id: &str, content: &str, days: i64) -> Result<()> {
    sqlx::query("INSERT INTO documents (id, content, days) VALUES (?, ?, ?)")
        .bind(id)
        .bind(content)
        .bind(days)
        .execute(pool)
        .await?;
    Ok(())
}

pub fn handle_404() -> Html<String> {
    Html(
        crate::views::create_404_page().into_string(),
    )
}
