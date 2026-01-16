use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Html,
    routing::get,
    Router,
};
use std::fs;
use std::path::PathBuf;
use tracing::{error, info, warn};

use crate::core::share::ShareService;
use crate::AppState;

pub fn share_routes() -> Router<AppState> {
    Router::new().route("/:share_id", get(share_page))
}

pub async fn share_page(
    State(state): State<AppState>,
    Path(share_id): Path<String>,
    headers: HeaderMap,
) -> Result<Html<String>, StatusCode> {
    // Removed client IP extraction
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("Unknown");
    
    info!(
        "🌐 Share page request - ID: {}",
        share_id
    );
    
    let share_service = ShareService::new(state.db.clone());
    
    // Verify share exists
    match share_service.get(&share_id).await {
        Ok(Some(_share)) => {
            info!(
                "✅ Share page rendered successfully - ID: {}",
                share_id
            );
            
            // Return HTML page using template
            let html = generate_share_page(&share_id)?;
            Ok(Html(html))
        }
        Ok(None) => {
            warn!(
                "⚠️ Share not found - ID: {} - User-Agent: {}",
                share_id, user_agent
            );
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!(
                 "❌ Error checking share - ID: {} - Error: {}",
                 share_id, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

fn generate_share_page(share_id: &str) -> Result<String, StatusCode> {
    let template_path = PathBuf::from("templates/share.html");

    // Read HTML template file
    let template_content = fs::read_to_string(template_path)
        .map_err(|e| {
            error!("Failed to read HTML template: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // HTML escape the share_id to prevent XSS
    let escaped_share_id = share_id
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;");

    // Replace {{share_id}} placeholder with escaped share_id
    let html = template_content.replace("{{share_id}}", &escaped_share_id);

    Ok(html)
}

