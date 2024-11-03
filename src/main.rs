// main.rs
use axum::{
    routing::get,
    response::{Html, IntoResponse},
    Router,
};
use maud::{html, Markup};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(render));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn render_ui() -> Markup {
    // Parse the embedded template string into Maud markup
    html! {
        head {
            title { "Mdow - A Markdown Meadow" }
            link rel="stylesheet" href="https://yree.io/mold/assets/css/main.css";
        }
        body {
            div class="w" {
                h1 { "Mdow" }
                p { "A Markdown Meadow" }
                
                div class="grid" {
                    button { "View MD" }
                    button { "Copy" }
                    button { "Share" }
                }
                
                div {
                    textarea placeholder="Enter your markdown here..." style="width: 100%" {}
                }
            }
        }
     }
}

async fn render() -> impl IntoResponse {
    Html(render_ui().into_string())
}
