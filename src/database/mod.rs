use sqlx::SqlitePool;
use std::sync::Arc;

pub type DbPool = Arc<SqlitePool>;

pub async fn create_pool(database_url: &str) -> anyhow::Result<SqlitePool> {
    let pool = SqlitePool::connect(database_url).await?;
    
    // Enable foreign key constraints
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;
    
    // Enable WAL mode for better concurrency
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;
    
    Ok(pool)
}