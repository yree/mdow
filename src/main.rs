// main.rs
use axum::{
    routing::{get, post},
    response::{Html, IntoResponse},
    Router,
    extract::Form,
};
use maud::{html, Markup};
use std::net::SocketAddr;
use serde::Deserialize;
use pulldown_cmark::{Parser, Options, html::push_html};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(render))
        .route("/preview", post(preview_markdown));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Deserialize)]
struct MarkdownInput {
    content: String,
}

fn render_ui() -> Markup {
    html! {
        head {
            title { "Mdow 🌾" }
            link rel="stylesheet" href="https://yree.io/mold/assets/css/main.css";
            script src="https://unpkg.com/htmx.org@1.9.10" {}
        }
        body {
            div class="w" {
                h1 { "Mdow 🌾" }
                p { 
                    b {"A meadow for your markdown files."}
                }
                p { "Enter your markdown, preview it, and share it with others. Shared links are valid for a month." }
                form {
                    div class="grid" {
                        button type="button" hx-post="/preview" hx-trigger="click" hx-target="#preview-area" hx-swap="innerHTML" hx-include="#markdown-input" { "Preview" }
                        button { "Share" }
                    }
                    div {
                        textarea id="markdown-input" name="content" placeholder="Enter your markdown..." style="width: 100%" {}
                    }
                    div id="preview-area" class="markdown-preview" {}
                }
            }
        }
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
    
    Html(html_output)
}

async fn render() -> impl IntoResponse {
    Html(render_ui().into_string())
}
