// main.rs
use axum::{
    extract::{Form, Path, Query, State},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use maud::{html, Markup, PreEscaped};
use std::net::SocketAddr;
use serde::Deserialize;
use pulldown_cmark::{Parser, Options, html::push_html};
use html_escape::encode_text;
use uuid::Uuid;
use std::str::FromStr;
use std::time::Duration;
use chrono::{DateTime, Utc};

#[derive(Deserialize)]
struct MarkdownInput {
    content: String,
}

#[derive(sqlx::FromRow)]
struct MarkdownDocument {
    id: String,
    content: String,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct RenderParams {
    content: Option<String>,
}

const DEFAULT_PORT: u16 = 8081;
const DEFAULT_DB_PATH: &str = "sqlite:data/database.db";
const DOCUMENT_EXPIRY_DAYS: i64 = 30;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn create_html_head(page_title: Option<&str>) -> Markup {
    html! {
        head {
            title { (page_title.unwrap_or("mdow")) }
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🌾</text></svg>";
            link rel="stylesheet" href="https://yree.io/mold/assets/css/main.css";
            script src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js" async="" {}
            script src="https://unpkg.com/htmx.org@1.9.10" {}
            script src="https://unpkg.com/hyperscript.org@0.9.12" {}
        }
    }
}

fn create_page_footer() -> Markup {
    html! {
        footer {
            div class="w" {
                p { a href="https://yree.io/mdow" { "mdow" } " 🌾 :: a " a href="https://yree.io" { "Yree" } " product ♥" }
            }
        }
    }
}

async fn create_markdown_editor_page(initial_content: &str) -> Markup {
    html! {
        (create_html_head(None))
        body a="auto" {
            main class="content" aria-label="Content" {
                div class="w" {
                    h1 { "mdow 🌾" }
                    p { dfn {"\"A meadow for your " b {"markdown on web."} "\"" } }
                    p { "Enter your markdown, preview it, and share it." }
                    div class="grid" {
                        button 
                            id="preview-button" 
                            hx-post="/preview" 
                            hx-trigger="click" 
                            hx-target="#markdown-input"
                            hx-swap="outerHTML"
                            hx-include="#markdown-input" 
                            hx-validate="true"
                            hx-disabled-elt="this" 
                            _="on htmx:afterRequest 
                               hide me 
                               show #edit-button"
                            { "Preview" }
                        button
                            id="edit-button" 
                            hx-post="/edit" 
                            hx-trigger="click" 
                            hx-target="#markdown-preview"
                            hx-swap="outerHTML" 
                            hx-include="#markdown-preview" 
                            style="display: none;" 
                            _="on click 
                               hide me 
                               show #preview-button"
                            { "Edit" }
                        button 
                            id="share-button"
                            hx-post="/share" 
                            hx-trigger="click" 
                            hx-include="[name='content']" 
                            hx-validate="true"
                            hx-disabled-elt="this" 
                            { "Share" }
                    }
                    textarea 
                        id="markdown-input" 
                        name="content" 
                        placeholder=(if initial_content.is_empty() { "Enter your markdown..." } else { "" }) 
                        style="width: 100%; height: calc(100vh - 275px); resize: none;"
                        required="required" {
                        @if !initial_content.is_empty() {
                            (initial_content)
                        }
                    }
                }
            }
        }
        (create_page_footer())
    }
}

fn convert_markdown_to_html(markdown_content: &str) -> String {
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

async fn handle_preview_request(Form(input): Form<MarkdownInput>) -> impl IntoResponse {
    let html_output = convert_markdown_to_html(&input.content);

    let preview_markup = html! {
        div id="markdown-preview" {
        br;
            input type="hidden" name="content" value=(encode_text(&input.content));
            (PreEscaped(html_output))
        }
        script { "MathJax.typeset();" }
    };

    Html(preview_markup.into_string())
}

async fn handle_edit_request(Form(input): Form<MarkdownInput>) -> impl IntoResponse {
    let edit_markup = html! {
        textarea id="markdown-input" name="content" placeholder="Enter your markdown..." style="width: 100%; height: calc(100vh - 275px); resize: none;" {
            (input.content)
        }
    };
    Html(edit_markup.into_string())
}

async fn handle_share_request(
    State(pool): State<SqlitePool>,
    Form(input): Form<MarkdownInput>,
) -> impl IntoResponse {
    let document_id = generate_short_uuid();
    let creation_time = Utc::now();
    let expiration_time = creation_time + chrono::Duration::days(DOCUMENT_EXPIRY_DAYS);

    save_markdown_document(&pool, &document_id, &input.content, creation_time, expiration_time).await;

    create_htmx_redirect_response(&document_id)
}

fn generate_short_uuid() -> String {
    Uuid::new_v4().to_string()[..7].to_string()
}

async fn save_markdown_document(
    pool: &SqlitePool, 
    id: &str, 
    content: &str, 
    created_at: DateTime<Utc>, 
    expires_at: DateTime<Utc>
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

async fn render(params: Option<Query<RenderParams>>) -> impl IntoResponse {
    let content = params
        .and_then(|p| p.0.content)
        .unwrap_or_else(|| "".to_string());
    
    let markup = create_markdown_editor_page(&content).await;
    Html(markup.into_string())
}

async fn view_shared(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let doc = sqlx::query_as::<_, MarkdownDocument>(
        "SELECT * FROM markdown_documents WHERE id = ? AND expires_at > datetime('now')"
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .expect("Failed to fetch document");

    match doc {
        Some(doc) => {
            let html_output = convert_markdown_to_html(&doc.content);
            // Extract title from first h1 tag or use default
            let title = html_output
                .find("<h1>")
                .and_then(|start| {
                    html_output[start..]
                        .find("</h1>")
                        .map(|end| &html_output[start + 4..start + end])
                });

            // Render the shared view
            let markup = html! {
                (create_html_head(title.as_deref()))
                body a="auto" {
                    main class="content" aria-label="Content" {
                        div class="w" {
                            (PreEscaped(html_output))
                        }
                    }
                }
                footer {
                    div class="w" {
                        {
                            p { 
                                "created on " (doc.created_at.format("%Y-%m-%d")) 
                            }
                            p {
                                a href=(format!("/?content={}", urlencoding::encode(&doc.content))) { "edit" }
                                " in "
                                a href="/" { "mdow" } 
                                " 🌾" 
                            }
                        }
                    }
                }
                script { "MathJax.typeset();" }
            };
            Html(markup.into_string())
        },
        None => handle_404()
    }
}

async fn debug_db(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let docs = sqlx::query_as::<_, MarkdownDocument>(
        "SELECT * FROM markdown_documents ORDER BY created_at DESC LIMIT 5"
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let debug_markup = html! {
        div {
            h2 { "Recent Documents" }
            @for doc in docs {
                div style="margin-bottom: 2ch; padding: 1ch; border: 1px solid #ccc;" {
                    p { "ID: " (doc.id) }
                    p { "Created: " (doc.created_at.format("%Y-%m-%d")) }
                    p { "Expires: " (doc.expires_at.format("%Y-%m-%d")) }
                    p { "Content: " (doc.content) }
                }
            }
        }
    };

    Html(debug_markup.into_string())
}

fn handle_404() -> Html<String> {
    Html(html! {
        (create_html_head(Some("404")))
        body a="auto" {
            main class="content" aria-label="Content" {
                div class="w" {
                    h1 { "404 - Page Not Found" }
                    p { "The page you're looking for doesn't exist." }
                    p { a href="/" { "Return to homepage" } }
                }
            }
        }
        (create_page_footer())
    }.into_string())
}

fn create_htmx_redirect_response(document_id: &str) -> impl IntoResponse {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("hx-redirect", format!("/view/{}", document_id).parse().unwrap());
    (headers, "")
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize database connection
    let pool = setup_database().await?;
    
    // Setup router
    let app = setup_router(pool);

    // Start server
    let addr = get_server_addr();
    println!("Listening on {}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn setup_database() -> Result<SqlitePool> {
    let db_path = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
    
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            SqliteConnectOptions::from_str(&db_path)?
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .busy_timeout(Duration::from_secs(30))
        )
        .await?;

    // Initialize schema
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS markdown_documents (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            created_at DATETIME NOT NULL,
            expires_at DATETIME NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

fn setup_router(pool: SqlitePool) -> Router {
    Router::new()
        .route("/", get(render))
        .route("/preview", post(handle_preview_request))
        .route("/edit", post(handle_edit_request))
        .route("/share", post(handle_share_request))
        .route("/view/:id", get(view_shared))
        .route("/debug", get(debug_db))
        .fallback(|| async { handle_404() })
        .with_state(pool)
}

fn get_server_addr() -> SocketAddr {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    
    SocketAddr::from(([0, 0, 0, 0], port))
}
