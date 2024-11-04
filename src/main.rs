// main.rs
use axum::{
    routing::{get, post},
    response::{Html, IntoResponse},
    Router,
    extract::Form,
    extract::State,
    extract::Path,
};
use maud::{html, Markup, PreEscaped};
use std::net::SocketAddr;
use serde::Deserialize;
use pulldown_cmark::{Parser, Options, html::push_html};
use html_escape::encode_text;
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};

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

fn render_ui() -> Markup {
    html! {
        head {
            title { "mdow 🌾" }
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            link rel="stylesheet" href="https://yree.io/mold/assets/css/main.css";
            script src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js" async="" {}
            script src="https://unpkg.com/htmx.org@1.9.10" {}
        }
        body a="auto" {
            main class="content" aria-label="Content" {
                div class="w" {
                    h1 { "mdow 🌾" }
                    p {
                        b {"A meadow for your markdown files."}
                    }
                    p { "Enter your markdown, preview it, and share it (link valid for a month)." }
                    div {
                        div class="grid" {
                            button id="preview-button" type="button" hx-post="/preview" hx-trigger="click" hx-target="#content-area" hx-swap="innerHTML" hx-include="#markdown-input" { "Preview" }
                            button id="edit-button" type="button" hx-post="/edit" hx-trigger="click" hx-target="#content-area" hx-swap="innerHTML" hx-include="#markdown-preview" style="display: none;" { "Edit" }
                            button hx-post="/share" hx-trigger="click" hx-target="#share-result" hx-include="#markdown-input" { "Share" }
                        }
                        div id="content-area" {
                            textarea id="markdown-input" name="content" placeholder="Enter your markdown..." style="width: 100%; height: calc(100vh - 275px); resize: none;" {}
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
        div id="markdown-preview" style="border: .2ch solid #000; padding: 2ch; height: calc(100vh - 275px); overflow-y: auto;" {
            input type="hidden" name="content" value=(encode_text(&input.content));
            (PreEscaped(html_output))
        }
        script {
            "document.getElementById('preview-button').style.display = 'none';"
            "document.getElementById('edit-button').style.display = 'block';"
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
        script {
            "document.getElementById('preview-button').style.display = 'block';"
            "document.getElementById('edit-button').style.display = 'none';"
        }
    };
    Html(edit_markup.into_string())
}

async fn render() -> impl IntoResponse {
    Html(render_ui().into_string())
}

async fn share_markdown(
    State(pool): State<SqlitePool>,
    Form(input): Form<MarkdownInput>,
) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let expires_at = now + Duration::days(30);

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

    // Generate share URL and return it
    let share_markup = html! {
        div {
            p { "Your document has been shared! Link valid for 30 days:" }
            input type="text" 
                  value=(format!("/view/{}", id))
                  readonly="readonly"
                  style="width: 100%; margin-top: 1ch;"
                  onclick="this.select()";
        }
    };

    Html(share_markup.into_string())
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
                    title { "mdow 🌾 - Shared Document" }
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    link rel="stylesheet" href="https://yree.io/mold/assets/css/main.css";
                    script src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js" async="" {}
                }
                body a="auto" {
                    main class="content" aria-label="Content" {
                        div class="w" {
                            h1 { "mdow 🌾" }
                            div style="border: .2ch solid #000; padding: 2ch;" {
                                (PreEscaped(html_output))
                            }
                        }
                    }
                }
                script { "MathJax.typeset();" }
            };
            Html(markup.into_string())
        },
        None => Html("<h1>Document not found or expired</h1>".to_string()),
    }
}

async fn debug_db(State(pool): State<SqlitePool>) -> impl IntoResponse {
    println!("debug db");
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
                    p { "Created: " (doc.created_at) }
                    p { "Expires: " (doc.expires_at) }
                    p { "Content: " (doc.content) }
                }
            }
        }
    };

    Html(debug_markup.into_string())
}

#[tokio::main]
async fn main() {
    // Initialize the database pool
    let pool = SqlitePool::connect("sqlite:database.db")
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
        .with_state(pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
