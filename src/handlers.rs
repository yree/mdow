use maud::{html, PreEscaped};
use axum::{
    extract::{ConnectInfo, Form, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Extension,
};
use sqlx::SqlitePool;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use crate::models::{CountdownParams, Document, LockoutParams, MarkdownInput, RenderParams, UnlockInput};
use crate::views::{create_lockout_fragment, create_markdown_editor_page, create_markdown_viewer_page, create_password_prompt_page, create_unlock_form_fragment};
use crate::utils::{handle_404, save_document, hash_password, verify_password, generate_short_uuid, create_htmx_redirect_response, convert_markdown_to_html, RateLimiter, ViewTracker};

const EXPIRY_SQL: &str =
    "SELECT * FROM documents WHERE id = ? AND datetime(created_at, '+' || days || ' days') > datetime('now')";

fn get_client_ip(headers: &HeaderMap, addr: SocketAddr) -> IpAddr {
    let ip = headers.get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(addr.ip());
    // Normalize IPv6 loopback and IPv4-mapped addresses to IPv4
    match ip {
        IpAddr::V6(v6) if v6 == Ipv6Addr::LOCALHOST => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(IpAddr::V6(v6)),
        other => other,
    }
}

pub async fn handle_main_request(
    State(pool): State<SqlitePool>,
    params: Option<Query<RenderParams>>,
) -> impl IntoResponse {
    let id = params.as_ref().and_then(|p| p.0.id.as_deref().map(str::to_owned));

    if let Some(ref id) = id {
        let doc = sqlx::query_as::<_, Document>(EXPIRY_SQL)
            .bind(id)
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten();

        if let Some(doc) = doc {
            if doc.password.is_some() {
                return Html(create_password_prompt_page(id, "edit", false, None).into_string());
            }
            return Html(create_markdown_editor_page(&doc.content).into_string());
        }
    }

    let content = params.and_then(|p| p.0.content).unwrap_or_default();
    Html(create_markdown_editor_page(&content).into_string())
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
    let days = input.days.unwrap_or(31).clamp(1, 365);
    let password = input.password
        .map(|p| p.trim().to_owned())
        .filter(|p| !p.is_empty())
        .map(|p| hash_password(&p))
        .transpose();

    let password = match password {
        Ok(p) => p,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to hash password").into_response(),
    };

    let tracking = input.tracking.as_deref() == Some("on");

    if save_document(&pool, &document_id, &input.content, days, password, tracking).await.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save document").into_response();
    }

    create_htmx_redirect_response(&document_id).into_response()
}

pub async fn handle_view_request(
    State(pool): State<SqlitePool>,
    Extension(view_tracker): Extension<Arc<ViewTracker>>,
    Extension(rate_limiter): Extension<Arc<RateLimiter>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let doc = sqlx::query_as::<_, Document>(EXPIRY_SQL)
        .bind(&id)
        .fetch_optional(&pool)
        .await;

    match doc {
        Ok(Some(doc)) if doc.password.is_some() => {
            let ip = get_client_ip(&request_headers, addr);
            let lockout = rate_limiter.lockout_info(ip, &id);
            Html(create_password_prompt_page(&id, "view", false, lockout).into_string())
        }
        Ok(Some(doc)) => {
            record_view(&pool, &view_tracker, &doc, get_client_ip(&request_headers, addr)).await;
            Html(create_markdown_viewer_page(&doc).into_string())
        }
        _ => handle_404(),
    }
}

async fn record_view(pool: &SqlitePool, tracker: &ViewTracker, doc: &Document, ip: IpAddr) {
    if doc.tracking && tracker.record(&doc.id, ip) {
        let _ = sqlx::query("UPDATE documents SET views = views + 1 WHERE id = ?")
            .bind(&doc.id)
            .execute(pool)
            .await;
    }
}

pub async fn handle_unlock_request(
    State(pool): State<SqlitePool>,
    Extension(rate_limiter): Extension<Arc<RateLimiter>>,
    Extension(view_tracker): Extension<Arc<ViewTracker>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_headers: HeaderMap,
    Path(id): Path<String>,
    Form(input): Form<UnlockInput>,
) -> Response {
    let ip = get_client_ip(&request_headers, addr);

    if let Some(lockout) = rate_limiter.lockout_info(ip, &id) {
        return Html(create_password_prompt_page(&id, &input.target, false, Some(lockout)).into_string()).into_response();
    }

    let doc = sqlx::query_as::<_, Document>(EXPIRY_SQL)
        .bind(&id)
        .fetch_optional(&pool)
        .await;

    let doc = match doc {
        Ok(Some(d)) => d,
        _ => return handle_404().into_response(),
    };

    let hash = match &doc.password {
        Some(h) => h,
        None => return handle_404().into_response(),
    };

    if !verify_password(input.password.trim(), hash) {
        rate_limiter.record_failure(ip, &id);
        let lockout = rate_limiter.lockout_info(ip, &id);
        return Html(create_password_prompt_page(&id, &input.target, true, lockout).into_string()).into_response();
    }

    rate_limiter.reset(ip, &id);

    if input.target == "edit" {
        Html(create_markdown_editor_page(&doc.content).into_string()).into_response()
    } else {
        record_view(&pool, &view_tracker, &doc, ip).await;
        Html(create_markdown_viewer_page(&doc).into_string()).into_response()
    }
}

pub async fn handle_lockout_request(
    State(pool): State<SqlitePool>,
    Extension(rate_limiter): Extension<Arc<RateLimiter>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_headers: HeaderMap,
    Path(id): Path<String>,
    Query(params): Query<LockoutParams>,
) -> Response {
    let ip = get_client_ip(&request_headers, addr);
    let target = params.target.as_deref().unwrap_or("view");

    if let Some(secs) = rate_limiter.secs_remaining(ip, &id) {
        return Html(create_lockout_fragment(&id, target, secs).into_string()).into_response();
    }

    let doc = sqlx::query_as::<_, Document>(EXPIRY_SQL)
        .bind(&id)
        .fetch_optional(&pool)
        .await;

    if let Ok(Some(_)) = doc {
        let mut response = Html(create_unlock_form_fragment(&id, target).into_string()).into_response();
        response.headers_mut().insert("HX-Trigger", "unlock".parse().unwrap());
        response
    } else {
        handle_404().into_response()
    }
}

pub async fn handle_countdown_request(
    Query(params): Query<CountdownParams>,
) -> Response {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    if params.expires > now {
        let remaining = params.expires - now;
        Html(create_lockout_fragment(&params.id, &params.target, remaining).into_string()).into_response()
    } else {
        let mut response = Html(create_unlock_form_fragment(&params.id, &params.target).into_string()).into_response();
        response.headers_mut().insert("HX-Trigger", "unlock".parse().unwrap());
        response
    }
}

pub async fn handle_stats_request(State(pool): State<SqlitePool>) -> impl IntoResponse {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents")
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let live: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM documents WHERE datetime(created_at, '+' || days || ' days') > datetime('now')",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(0);

    Html(format!("{total} docs shared, {live} still active."))
}

