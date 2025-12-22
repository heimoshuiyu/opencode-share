use sqlx::PgPool;
use std::sync::Arc;

pub type DbPool = Arc<PgPool>;

pub async fn create_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPool::connect(database_url).await?;
    
    // PostgreSQL has foreign key constraints enabled by default
    // and doesn't need WAL mode like SQLite
    
    Ok(pool)
}