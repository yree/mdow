// main.rs
use axum::{
    routing::{get, post},
    response::{Html, IntoResponse},
    Router,
    extract::Form,
};
use maud::{html, Markup, PreEscaped};
use std::net::SocketAddr;
use serde::Deserialize;
use pulldown_cmark::{Parser, Options, html::push_html};
use html_escape::encode_text;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(render))
        .route("/preview", post(preview_markdown))
        .route("/edit", post(edit_mode));

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
            script src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js" async="" {}
            script src="https://unpkg.com/htmx.org@1.9.10" {}
        }
        body {
            div class="w" {
                h1 { "Mdow 🌾" }
                p { 
                    b {"A meadow for your markdown files."}
                }
                p { "Enter your markdown, preview it, and share it with others." }
                p { "Shared links are valid for a month." }
                div {
                    div class="grid" {
                        button id="preview-button" type="button" hx-post="/preview" hx-trigger="click" hx-target="#content-area" hx-swap="innerHTML" hx-include="#markdown-input" { "Preview" }
                        button id="edit-button" type="button" hx-post="/edit" hx-trigger="click" hx-target="#content-area" hx-swap="innerHTML" hx-include="#markdown-preview" style="display: none;" { "Edit" }
                        button disabled { "Share (coming soon)" }
                    }
                    div id="content-area" {
                        textarea id="markdown-input" name="content" placeholder="Enter your markdown..." style="width: 100%; height: 100%;" {}
                    }
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
    
    let preview_markup = html! {

        div id="markdown-preview" {
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
        textarea id="markdown-input" name="content" placeholder="Enter your markdown..." style="width: 100%" {
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
