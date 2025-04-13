
use crate::Result;
use sqlx::{SqlitePool, SqlitePoolOptions, SqliteConnectOptions, SqliteJournalMode};
use std::time::Duration;

pub async fn setup_database() -> Result<SqlitePool> {
    let db_path = std::env::var("DATABASE_URL").unwrap_or_else(|_| super::DEFAULT_DB_PATH.to_string());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            SqliteConnectOptions::from_str(&db_path)?
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .busy_timeout(Duration::from_secs(30)),
        )
        .await?;

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
    .await?;

    Ok(pool)
}
