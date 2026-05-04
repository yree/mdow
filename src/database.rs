use crate::Result;
use sqlx::{sqlite::{SqlitePool, SqliteConnectOptions, SqliteJournalMode}, pool::PoolOptions as SqlitePoolOptions};
use std::str::FromStr;
use std::time::Duration;

pub async fn setup_database() -> Result<SqlitePool> {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| super::DEFAULT_DB_PATH.to_string());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            SqliteConnectOptions::from_str(&db_url)?
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .busy_timeout(Duration::from_secs(30)),
        )
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS documents (
            id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT (datetime('now')),
            days INTEGER NOT NULL DEFAULT 30,
            views INTEGER NOT NULL DEFAULT 0,
            password TEXT
        )
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
