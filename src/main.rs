// main.rs
use axum::{
    extract::{Form, Path, Query, State},
    http::{HeaderValue, StatusCode},
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

fn common_head(title: Option<&str>) -> Markup {
    html! {
        title { (title.unwrap_or("mdow")) }
        meta charset="utf-8";
        meta name="viewport" content="width=device-width, initial-scale=1";
        link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🌾</text></svg>";
        link rel="stylesheet" href="https://yree.io/mold/assets/css/main.css";
        script src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js" async="" {}
        script src="https://unpkg.com/htmx.org@1.9.10" {}
        script src="https://unpkg.com/hyperscript.org@0.9.12" {}
    }
}

async fn render_ui(content: &str) -> Markup {
    html! {
        head {
            (common_head(None))
        }
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
                        placeholder=(if content.is_empty() { "Enter your markdown..." } else { "" }) 
                        style="width: 100%; height: calc(100vh - 275px); resize: none;"
                        required="required" {
                        @if !content.is_empty() {
                            (content)
                        }
                    }
                }
            }
        }
        footer {
            div class="w" {
                {
                    p { a href="https://yree.io/mdow" { "mdow" } " 🌾 :: a " a href="https://yree.io" { "Yree" } " product ♥" }
                }
            }
        }
        div id="share-result" {}
    }
}

fn render_markdown(content: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    
    let parser = Parser::new_ext(content, options);
    let mut html_output = String::new();
    push_html(&mut html_output, parser);
    
    html_output
        .replace("<pre>", "<div class=\"highlighter-rouge\"><pre>")
        .replace("</pre>", "</pre></div>")
}

async fn preview_markdown(Form(input): Form<MarkdownInput>) -> impl IntoResponse {
    let html_output = render_markdown(&input.content);

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

async fn edit_mode(Form(input): Form<MarkdownInput>) -> impl IntoResponse {
    let edit_markup = html! {
        textarea id="markdown-input" name="content" placeholder="Enter your markdown..." style="width: 100%; height: calc(100vh - 275px); resize: none;" {
            (input.content)
        }
    };
    Html(edit_markup.into_string())
}

async fn render(params: Option<Query<RenderParams>>) -> impl IntoResponse {
    let content = params
        .and_then(|p| p.0.content)
        .unwrap_or_else(|| "".to_string());
    
    let markup = render_ui(&content).await;
    Html(markup.into_string())
}

async fn share_markdown(
    State(pool): State<SqlitePool>,
    Form(input): Form<MarkdownInput>,
) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string()[..7].to_string();
    let now = Utc::now();
    let expires_at = now + chrono::Duration::days(DOCUMENT_EXPIRY_DAYS);

    // Store the document
    sqlx::query(
        r#"
        INSERT INTO markdown_documents (id, content, created_at, expires_at)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&input.content)
    .bind(now)
    .bind(expires_at)
    .execute(&pool)
    .await
    .expect("Failed to save document");

    // Use HX-Redirect header for HTMX to handle the redirect
    (
        [(axum::http::header::HeaderName::from_static("hx-redirect"), 
          format!("/view/{}", id).parse::<HeaderValue>().unwrap())],
        "Redirecting..."
    )
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
            let html_output = render_markdown(&doc.content);
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
                head {
                    (common_head(title.as_deref()))
                }
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
        None => {
            let markup = html! {
                head {
                    (common_head(Some("404")))
                }
                body a="auto" {
                    main class="content" aria-label="Content" {
                    div class="w" {
                        h1 { "Document not found or expired" }
                        p { "The page you're looking for doesn't exist." }
                        p { a href="/" { "Return to homepage" } }
                        }
                    }
                }
                footer {
                    div class="w" {
                        p { a href="https://github.com/yree/mdow" { "@yree/mdow" } " :: A " a href="https://yree.io" { "Yree" } " product ♥" }
                    }
                }
            };
            Html(markup.into_string())
        }
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

async fn handle_404() -> impl IntoResponse {
    let markup = html! {
        head {
            (common_head(None))
        }
        body a="auto" {
            main class="content" aria-label="Content" {
                div class="w" {
                    h1 { "404 - Page Not Found" }
                    p { "The page you're looking for doesn't exist." }
                    p { a href="/" { "Return to homepage" } }
                }
            }
        }
        footer {
            div class="w" {
                {
                    p { a href="https://github.com/yree/mdow" { "@yree/mdow" } " :: A " a href="https://yree.io" { "Yree" } " product ♥" }
                }
            }
        }
    };

    (StatusCode::NOT_FOUND, Html(markup.into_string()))
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
        .route("/preview", post(preview_markdown))
        .route("/edit", post(edit_mode))
        .route("/share", post(share_markdown))
        .route("/view/:id", get(view_shared))
        .route("/debug", get(debug_db))
        .fallback(handle_404)
        .with_state(pool)
}

fn get_server_addr() -> SocketAddr {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    
    SocketAddr::from(([0, 0, 0, 0], port))
}
