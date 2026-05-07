use std::str::FromStr as _;

use anyhow::{Context as _, Result};

pub mod webhook_tasks;

pub struct Sqlite {
    //
    pool: sqlx::SqlitePool,
}

impl Sqlite {
    pub async fn new(path: &str) -> Result<Self> {
        let pool = sqlx::SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::from_str(path)
                .with_context(|| format!("invalid database path {}", path))?
                .pragma("foreign_keys", "ON"),
        )
        .await
        .with_context(|| format!("failed to open database at {}", path))?;

        Ok(Sqlite { pool })
    }
}
