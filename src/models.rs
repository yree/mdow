use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct MarkdownInput {
    pub content: String,
    pub days: Option<i64>,
    pub password: Option<String>,
    pub tracking: Option<String>,
}

#[derive(Deserialize)]
pub struct UnlockInput {
    pub password: String,
    pub target: String,
}

#[derive(sqlx::FromRow)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub days: i64,
    pub views: i64,
    pub password: Option<String>,
    pub tracking: bool,
}

#[derive(Deserialize)]
pub struct RenderParams {
    pub content: Option<String>,
    pub id: Option<String>,
}
