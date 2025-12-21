use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{delete, get, post},
    Router,
};
use serde::Deserialize;
use tracing::{debug, error, info};

use crate::{
    core::share::ShareService,
    models::{
        CreateShareRequest, CreateShareResponse, RemoveShareRequest, SyncShareRequest, ShareData,
    },
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct ShareQuery {
    sessionID: String,
}

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/share", post(create_share))
        .route("/share/:share_id/sync", post(sync_share))
        .route("/share/:share_id/data", get(get_share_data))
        .route("/share/:share_id", delete(remove_share))
}

pub async fn create_share(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateShareRequest>,
) -> Result<Json<CreateShareResponse>, StatusCode> {
    // 提取客户端信息用于详细日志
    let client_ip = get_client_info(&headers);
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("Unknown");
    
    let session_id = request.session_id.clone();
    info!(
        "🆕 Creating share - SessionID: {} - IP: {} - User-Agent: {}",
        session_id,
        client_ip,
        user_agent
    );
    
    let share_service = ShareService::new(state.db.clone());
    
    match share_service.create(request.session_id).await {
        Ok(share) => {
            // Build URL similar to original
            let protocol = headers
                .get("x-forwarded-proto")
                .or_else(|| headers.get("x-forwarded-protocol"))
                .and_then(|h| h.to_str().ok())
                .unwrap_or("https");
            
            let host = headers
                .get("x-forwarded-host")
                .or_else(|| headers.get("host"))
                .and_then(|h| h.to_str().ok())
                .unwrap_or("localhost:3000");

            let url = format!("{protocol}://{host}/share/{}", share.id);
            
            info!(
                "✅ Share created successfully - ID: {} - URL: {} - IP: {}",
                share.id, url, client_ip
            );
            
            Ok(Json(CreateShareResponse {
                id: share.id,
                secret: share.secret,
                url,
            }))
        }
        Err(e) => {
            error!(
                "❌ Failed to create share - SessionID: {} - Error: {} - IP: {}",
                session_id, e, client_ip
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn sync_share(
    State(state): State<AppState>,
    Path(share_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<SyncShareRequest>,
) -> Result<(), StatusCode> {
    // 提取客户端信息用于详细日志
    let client_ip = get_client_info(&headers);
    let data_size = request.data.len();
    
    info!(
        "🔄 Syncing data to share - ID: {} - Data size: {} items - IP: {}",
        share_id, data_size, client_ip
    );
    
    let share_service = ShareService::new(state.db.clone());
    
    match share_service.sync(&share_id, &request.secret, request.data).await {
        Ok(_) => {
            info!(
                "✅ Successfully synced data to share - ID: {} - Data items: {} - IP: {}",
                share_id, data_size, client_ip
            );
            Ok(())
        }
        Err(e) => {
            error!(
                "❌ Failed to sync share - ID: {} - Error: {} - IP: {}",
                share_id, e, client_ip
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_share_data(
    State(state): State<AppState>,
    Path(share_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Vec<ShareData>>, StatusCode> {
    // 提取客户端信息用于详细日志
    let client_ip = get_client_info(&headers);
    
    info!(
        "📖 Retrieving share data - ID: {} - IP: {}",
        share_id, client_ip
    );
    
    let share_service = ShareService::new(state.db.clone());
    
    match share_service.get_data(&share_id).await {
        Ok(data) => {
            info!(
                "✅ Retrieved share data - ID: {} - Data items: {} - IP: {}",
                share_id, data.len(), client_ip
            );
            debug!("Share {} data preview: {:?}", share_id, data);
            Ok(Json(data))
        }
        Err(e) => {
            error!(
                "❌ Failed to get share data - ID: {} - Error: {} - IP: {}",
                share_id, e, client_ip
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn remove_share(
    State(state): State<AppState>,
    Path(share_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<RemoveShareRequest>,
) -> Result<(), StatusCode> {
    // 提取客户端信息用于详细日志
    let client_ip = get_client_info(&headers);
    
    info!(
        "🗑️ Removing share - ID: {} - IP: {}",
        share_id, client_ip
    );
    
    let share_service = ShareService::new(state.db.clone());
    
    match share_service.remove(&share_id, &request.secret).await {
        Ok(_) => {
            info!(
                "✅ Successfully removed share - ID: {} - IP: {}",
                share_id, client_ip
            );
            Ok(())
        }
        Err(e) => {
            error!(
                "❌ Failed to remove share - ID: {} - Error: {} - IP: {}",
                share_id, e, client_ip
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
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