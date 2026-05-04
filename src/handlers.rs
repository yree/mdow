use maud::{html, PreEscaped};
use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use chrono::Utc;
use sqlx::SqlitePool;
use crate::models::{MarkdownDocument, MarkdownInput, RenderParams};
use crate::views::{create_markdown_editor_page, create_markdown_viewer_page};
use crate::utils::{handle_404, save_markdown_document, generate_short_uuid, create_htmx_redirect_response, convert_markdown_to_html};

pub async fn handle_main_request(params: Option<Query<RenderParams>>) -> impl IntoResponse {
    let content = params
        .and_then(|p| p.0.content)
        .unwrap_or_else(|| "".to_string());

    let markup = create_markdown_editor_page(&content);
    Html(markup.into_string())
}

pub async fn handle_preview_request(Form(input): Form<MarkdownInput>) -> impl IntoResponse {
    let html_output = convert_markdown_to_html(&input.content);
    let markup = html! {
        div _="on load call MathJax.typeset()" {
            (PreEscaped(html_output))
        }
    };
    Html(markup.into_string())
}

pub async fn handle_share_request(
    State(pool): State<SqlitePool>,
    Form(input): Form<MarkdownInput>,
) -> Response {
    let document_id = generate_short_uuid();
    let creation_time = Utc::now();
    let expiration_time = creation_time + chrono::Duration::days(super::DOCUMENT_EXPIRY_DAYS);

    if save_markdown_document(
        &pool,
        &document_id,
        &input.content,
        creation_time,
        expiration_time,
    )
    .await
    .is_err()
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save document").into_response();
    }

    create_htmx_redirect_response(&document_id).into_response()
}

pub async fn handle_debug_request(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let docs = sqlx::query_as::<_, MarkdownDocument>(
        "SELECT id, content, created_at FROM markdown_documents ORDER BY created_at DESC LIMIT 5",
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let markup = html! {
        div {
            h2 { "Recent Documents" }
            @for doc in &docs {
                div style="margin-bottom: 2ch; padding: 1ch; border: .2ch solid #000;" {
                    p { "ID: " (doc.id) }
                    p { "Created: " (doc.created_at.format("%Y-%m-%d")) }
                    p { "Content: " (doc.content) }
                }
            }
        }
    };

    Html(markup.into_string())
}

pub async fn handle_view_request(
    State(pool): State<SqlitePool>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let doc = sqlx::query_as::<_, MarkdownDocument>(
        "SELECT id, content, created_at FROM markdown_documents WHERE id = ? AND expires_at > datetime('now')",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await;

    match doc {
        Ok(Some(doc)) => {
            let markup = create_markdown_viewer_page(&doc);
            Html(markup.into_string())
        }
        _ => handle_404(),
    }
}
