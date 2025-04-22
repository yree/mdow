use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct MarkdownInput {
    pub content: String,
}

#[derive(sqlx::FromRow)]
pub struct MarkdownDocument {
    pub id: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub expires_at:  DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct RenderParams {
    pub content: Option<String>,
}
