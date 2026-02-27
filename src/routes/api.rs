use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{delete, get, post},
    Router,
};
use serde_json::Value;
use tracing::{debug, error, info};

use crate::{
    core::share::ShareService,
    models::{CreateShareRequest, CreateShareResponse, RemoveShareRequest, SyncShareRequest},
    AppState,
};



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
    // Removed client IP extraction
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("Unknown");
    
    let session_id = request.session_id.clone();
    info!(
        "🆕 Creating share - SessionID: {} - User-Agent: {}",
        session_id,
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
                "✅ Share created successfully - ID: {} - URL: {}",
                share.id, url
            );
            
            Ok(Json(CreateShareResponse {
                id: share.id,
                secret: share.secret,
                url,
            }))
        }
        Err(e) => {
            error!(
                 "❌ Failed to create share - SessionID: {} - Error: {}",
                 session_id, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn sync_share(
    State(state): State<AppState>,
    Path(share_id): Path<String>,
    Json(request): Json<SyncShareRequest>,
) -> Result<(), StatusCode> {
    // Removed client IP extraction
    let data_size = request.data.len();
    
    info!(
        "🔄 Syncing data to share - ID: {} - Data size: {} items",
        share_id, data_size
    );
    
    let share_service = ShareService::new(state.db.clone());
    
    match share_service.sync(&share_id, &request.secret, request.data).await {
        Ok(_) => {
            info!(
                "✅ Successfully synced data to share - ID: {} - Data items: {}",
                share_id, data_size
            );
            Ok(())
        }
        Err(e) => {
            error!(
                 "❌ Failed to sync share - ID: {} - Error: {}",
                 share_id, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_share_data(
    State(state): State<AppState>,
    Path(share_id): Path<String>,
) -> Result<Json<Vec<Value>>, StatusCode> {
    // Removed client IP extraction
    
    info!(
        "📖 Retrieving share data - ID: {}",
        share_id
    );
    
    let share_service = ShareService::new(state.db.clone());
    
    match share_service.get_data(&share_id).await {
        Ok(data) => {
            info!(
                "✅ Retrieved share data - ID: {} - Data items: {}",
                share_id, data.len()
            );
            debug!("Share {} data preview: {:?}", share_id, data);
            Ok(Json(data))
        }
        Err(e) => {
            error!(
                 "❌ Failed to get share data - ID: {} - Error: {}",
                 share_id, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn remove_share(
    State(state): State<AppState>,
    Path(share_id): Path<String>,
    Json(request): Json<RemoveShareRequest>,
) -> Result<(), StatusCode> {
    // Removed client IP extraction
    
    info!(
        "🗑️ Removing share - ID: {}",
        share_id
    );
    
    let share_service = ShareService::new(state.db.clone());
    
    match share_service.remove(&share_id, &request.secret).await {
        Ok(_) => {
            info!(
                "✅ Successfully removed share - ID: {}",
                share_id
            );
            Ok(())
        }
        Err(e) => {
            error!(
                 "❌ Failed to remove share - ID: {} - Error: {}",
                 share_id, e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

