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
    // 提取客户端信息用于详细日志
    let client_ip = get_client_info(&headers);
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("Unknown");
    
    info!(
        "🌐 Share page request - ID: {} - IP: {} - User-Agent: {}",
        share_id, client_ip, user_agent
    );
    
    let share_service = ShareService::new(state.db.clone());
    
    // Verify share exists
    match share_service.get(&share_id).await {
        Ok(Some(_share)) => {
            info!(
                "✅ Share page rendered successfully - ID: {} - IP: {}",
                share_id, client_ip
            );
            
            // Return HTML page using template
            let html = generate_share_page(&share_id)?;
            Ok(Html(html))
        }
        Ok(None) => {
            warn!(
                "⚠️ Share not found - ID: {} - IP: {} - User-Agent: {}",
                share_id, client_ip, user_agent
            );
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            error!(
                "❌ Error checking share - ID: {} - Error: {} - IP: {}",
                share_id, e, client_ip
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
    
    // Replace {{share_id}} placeholder with actual share_id
    let html = template_content.replace("{{share_id}}", share_id);
    
    Ok(html)
}

/// 从请求头中提取客户端IP地址
fn get_client_info(headers: &HeaderMap) -> String {
    // 尝试从各种头部获取真实IP
    headers
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|h| h.to_str().ok())
        })
        .or_else(|| {
            headers
                .get("cf-connecting-ip") // Cloudflare
                .and_then(|h| h.to_str().ok())
        })
        .or_else(|| {
            headers
                .get("x-client-ip")
                .and_then(|h| h.to_str().ok())
        })
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}