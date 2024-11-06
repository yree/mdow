// main.rs
use axum::{
    routing::{get, post},
    response::{Html, IntoResponse},
    Router,
    extract::Form,
    extract::State,
    extract::Path,
    http::HeaderValue,
    extract::Query,
    http::StatusCode,
};
use maud::{html, Markup, PreEscaped};
use std::net::SocketAddr;
use serde::Deserialize;
use pulldown_cmark::{Parser, Options, html::push_html};
use html_escape::encode_text;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions, SqliteConnectOptions, SqliteJournalMode};
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

fn common_head() -> Markup {
    html! {
        title { "mdow 🌾" }
        meta charset="utf-8";
        meta name="viewport" content="width=device-width, initial-scale=1";
        link rel="stylesheet" href="https://yree.io/mold/assets/css/main.css";
        script src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js" async="" {}
        script src="https://unpkg.com/htmx.org@1.9.10" {}
        script src="https://unpkg.com/hyperscript.org@0.9.12" {}
    }
}

async fn render_ui(content: &str) -> Markup {
    html! {
        head {
            (common_head())
        }
        body a="auto" {
            main class="content" aria-label="Content" {
                div class="w" {
                    h1 { "mdow 🌾" }
                    p {
                        b {"A meadow for your markdown files."}
                    }
                    p { "Enter your markdown, preview it, and share it." }
                    div {
                        div class="grid" {
                            button 
                                id="preview-button" 
                                hx-post="/preview" 
                                hx-trigger="click" 
                                hx-target="#content-area" 
                                hx-swap="innerHTML" 
                                hx-include="#markdown-input" 
                                hx-validate="true"
                                hx-disabled-elt="this" 
                                _="on click 
                                   hide me 
                                   show #edit-button"
                                { "Preview" }
                            button
                                id="edit-button" 
                                hx-post="/edit" 
                                hx-trigger="click" 
                                hx-target="#content-area" 
                                hx-swap="innerHTML" 
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
                        div id="content-area" {
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
            }
        }
        footer {
            div class="w" {
                {
                    p { a href="https://github.com/yree/mdow" { "@yree/mdow" } " :: A " a href="https://yree.io" { "Yree" } " product ♥" }
                }
            }
        }
        div id="share-result" {}
    }
}

async fn preview_markdown(Form(input): Form<MarkdownInput>) -> impl IntoResponse {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    
    let parser = Parser::new_ext(&input.content, options);
    let mut html_output = String::new();
    push_html(&mut html_output, parser);
    
    let html_output = html_output
        .replace(
            "<pre>",
            "<div class=\"highlighter-rouge\"><pre>"
        )
        .replace("</pre>", "</pre></div>");

    let preview_markup = html! {
        div id="markdown-preview" {
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
    let expires_at = now + chrono::Duration::days(30);

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
    // Fetch the document
    let doc = sqlx::query_as::<_, MarkdownDocument>(
        "SELECT * FROM markdown_documents WHERE id = ? AND expires_at > datetime('now')"
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .expect("Failed to fetch document");

    match doc {
        Some(doc) => {
            // Convert markdown to HTML (reuse your existing conversion logic)
            let mut options = Options::empty();
            options.insert(Options::ENABLE_TABLES);
            options.insert(Options::ENABLE_STRIKETHROUGH);
            options.insert(Options::ENABLE_TASKLISTS);
 
            let parser = Parser::new_ext(&doc.content, options);
            let mut html_output = String::new();
            push_html(&mut html_output, parser);

            let html_output = html_output
                .replace("<pre>", "<div class=\"highlighter-rouge\"><pre>")
                .replace("</pre>", "</pre></div>");

            // Render the shared view
            let markup = html! {
                head {
                    (common_head())
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
                                " :: " 
                                a href=(format!("/?content={}", urlencoding::encode(&doc.content))) { "edit" }
                                " :: "
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
                    (common_head())
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
            (common_head())
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
async fn main() {
    // Use environment variable with a default fallback for local development
    let db_path = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:data/database.db".to_string());
    
    // Configure SQLite connection pool with WAL mode and busy timeout
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            SqliteConnectOptions::from_str(&db_path)
                .unwrap()
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .busy_timeout(Duration::from_secs(30))
        )
        .await
        .expect("Failed to connect to database");

    // Create the table if it doesn't exist
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
    .await
    .expect("Failed to create table");

    let app = Router::new()
        .route("/", get(render))
        .route("/preview", post(preview_markdown))
        .route("/edit", post(edit_mode))
        .route("/share", post(share_markdown))
        .route("/view/:id", get(view_shared))
        .route("/debug", get(debug_db))
        .fallback(handle_404)
        .with_state(pool);

    // Get port from environment or default to 8081
    let port =
        std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8081);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
