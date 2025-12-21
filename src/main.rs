use axum::{
    middleware::from_fn_with_state,
    routing::get,
    Router,
};
use sqlx::SqlitePool;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

mod core;
mod database;
mod middleware;
mod models;
mod routes;

use routes::{api_routes, share_routes};
use middleware::access_log_middleware;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();
    
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "opencode_share=debug,tower_http=debug".into()),
        )
        .init();

    // Initialize database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| {
            let current_dir = std::env::current_dir().unwrap();
            let db_path = current_dir.join("opencode-share.db");
            format!("sqlite:{}", db_path.display())
        });
    
    println!("Using database: {}", database_url);
    let pool = SqlitePool::connect(&database_url).await?;
    
    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");
    
    let app_state = AppState { db: pool };

    // Build the application
    let app = Router::new()
        // Apply access log middleware to all routes
        .layer(from_fn_with_state(app_state.clone(), access_log_middleware))
        // API routes
        .nest("/api", api_routes())
        // Share pages
        .nest("/share", share_routes())
        // Static files
        .nest_service("/static", tower_http::services::ServeDir::new("static"))
        // Root route
        .route("/", get(index))
        // CORS
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3006").await?;
    info!("Server listening on {}", listener.local_addr()?);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn index() -> &'static str {
    info!("🏠 Home page requested");
    "Hello World"
}