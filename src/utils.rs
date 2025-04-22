use axum::response::IntoResponse;
use sqlx::SqlitePool;
use maud::{html, Markup, PreEscaped};
use pulldown_cmark::{html::push_html, Options, Parser};
use qrcode::{render::svg, QrCode};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use axum::response::Html;
use crate::models::MarkdownDocument;

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

pub fn create_html_head(page_title: Option<&str>) -> Markup {
    html! {
        head {
            title { (page_title.unwrap_or("mdow")) };

            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";

            meta name="title" content="mdow 🌾 | markdown on web";
            meta name="description" content="A meadow for your markdown on web. A lightweight, browser-based markdown editor and previewer that makes sharing markdown files as simple as sharing a link.";
            meta name="keywords" content="markdown editor, online markdown, markdown preview, markdown sharing, web markdown, browser markdown";

            meta name="application-name" content="mdow";
            meta name="mobile-web-app-capable" content="yes";
            meta name="apple-mobile-web-app-capable" content="yes";
            meta name="apple-mobile-web-app-title" content="mdow";
            meta name="apple-mobile-web-app-status-bar-style" content="default";
            meta name="theme-color" content="#ffffff" media="(prefers-color-scheme: light)";
            meta name="theme-color" content="#000000" media="(prefers-color-scheme: dark)";

            link rel="apple-touch-icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🌾</text></svg>";

            link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🌾</text></svg>";
            link rel="stylesheet" href="https://yree.io/mold/assets/css/main.css";

            script src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js" async="" {};
            script src="https://unpkg.com/htmx.org@1.9.10" {};
            script src="https://unpkg.com/hyperscript.org@0.9.12" {};

            script data-goatcounter="https://yree.goatcounter.com/count" async src="//gc.zgo.at/count.js" {};
        }
    }
}

pub fn create_page_footer() -> Markup {
    html! {
        footer {
            div class="w" {
                p { a href="https://yree.io/mdow" { "mdow" } " 🌾 — a " a href="https://yree.io" { "Yree" } " product ♥" }
            }
        }
    }
}

pub async fn create_markdown_editor_page(initial_content: &str) -> Markup {
    html! {
        (create_html_head(None));
        body a="auto" {
            main class="content" aria-label="Content" {
                div class="w" {
                    h1 { "mdow 🌾" }
                    p { dfn {"A meadow for your " b {"markdown on web."} } }
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
                            hx-disabled-elt="this"
                            _="on htmx:afterRequest
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
                        style="width: 100%; height: 30ch; resize: vertical"
                        required="required"
                        _=(if initial_content.is_empty() {
                            "on load
                                set my.value to (localStorage.getItem('markdownContent'))
                             on input
                                wait 500ms then
                                call localStorage.setItem('markdownContent', my.value)"
                        } else {
                            "on input
                                wait 500ms then
                                call localStorage.setItem('markdownContent', my.value)"
                        })
                        { (initial_content) }
                }
            }
        }
        (create_page_footer());
    }
}

pub fn create_markdown_viewer_page(doc: &MarkdownDocument) -> Markup {
    let html_output = convert_markdown_to_html(&doc.content);
    let page_title = extract_title_from_html(&html_output);

    html! {
        (create_html_head(page_title));
        body a="auto" {
            main class="content" aria-label="Content" {
                div class="w" id="markdown-view" _="on load call MathJax.typeset()" {
                    (PreEscaped(html_output))
                }
            }
            footer {
                div class="w grid" {
                    (PreEscaped(generate_qr_svg(&doc.id)))
                    div {
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
        }
    }
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
        html! {
            (create_html_head(Some("404")));
            body a="auto" {
                main class="content" aria-label="Content" {
                    div class="w" {
                        h1 { "404 - Page Not Found" }
                        p { "The page you're looking for doesn't exist." }
                        p { a href="/" { "Return to homepage" } }
                    }
                }
            }
            (create_page_footer());
        }
        .into_string(),
    )
}
