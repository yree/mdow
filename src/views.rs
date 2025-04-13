use maud::{html, PreEscaped};
use axum::response::{Html, IntoResponse};
use crate::{MarkdownDocument, MarkdownInput, RenderParams, clean, convert_markdown_to_html, create_markdown_editor_page, create_markdown_viewer_page, handle_404, save_markdown_document, generate_short_uuid, create_htmx_redirect_response};
use sqlx::SqlitePool;
use axum::extract::{Form, Path, Query, State};
use chrono::{DateTime, Utc};

pub async fn handle_main_request(params: Option<Query<RenderParams>>) -> impl IntoResponse {
    let content = params
        .and_then(|p| p.0.content)
        .unwrap_or_else(|| "".to_string());

    let markup = create_markdown_editor_page(&content).await;
    Html(markup.into_string())
}

pub async fn handle_preview_request(Form(input): Form<MarkdownInput>) -> impl IntoResponse {
    let sanitized_content = clean(&input.content);
    let html_output = convert_markdown_to_html(&sanitized_content);

    let preview_markup = html! {
        div id="markdown-preview" _="on load call MathJax.typeset()" {
            br;
            input type="hidden" name="content" value=(&input.content);
            (PreEscaped(html_output))
        }
    };

    Html(preview_markup.into_string())
}

pub async fn handle_edit_request(Form(input): Form<MarkdownInput>) -> impl IntoResponse {
    let edit_markup = html! {
        textarea id="markdown-input" name="content" placeholder="Enter your markdown..." style="width: 100%; height: calc(100vh - 275px); resize: none;" {
            (input.content)
        }
    };
    Html(edit_markup.into_string())
}

pub async fn handle_share_request(
    State(pool): State<SqlitePool>,
    Form(input): Form<MarkdownInput>,
) -> impl IntoResponse {
    let document_id = generate_short_uuid();
    let creation_time = Utc::now();
    let expiration_time = creation_time + chrono::Duration::days(super::DOCUMENT_EXPIRY_DAYS);

    let sanitized_content = clean(&input.content);

    save_markdown_document(
        &pool,
        &document_id,
        &sanitized_content,
        creation_time,
        expiration_time,
    )
    .await;

    create_htmx_redirect_response(&document_id)
}

pub async fn handle_view_request(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let doc = sqlx::query_as::<_, MarkdownDocument>(
        "SELECT * FROM markdown_documents WHERE id = ? AND expires_at > datetime('now')",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .expect("Failed to fetch document");

    match doc {
        Some(doc) => {
            let markup = create_markdown_viewer_page(&doc);
            Html(markup.into_string())
        }
        None => handle_404(),
    }
}

pub async fn handle_debug_request(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let docs = sqlx::query_as::<_, MarkdownDocument>(
        "SELECT * FROM markdown_documents ORDER BY created_at DESC LIMIT 5",
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
