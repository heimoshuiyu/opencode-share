use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Share {
    pub id: String,
    pub secret: String,
    pub session_id: String,
    pub data: Option<Value>, // JSONB field storing current state as array
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// ShareData is now an arbitrary JSON value.
/// Clients should include a `_key` field to identify how data should be merged.
/// Optional `_type` field can be used by clients for their own type identification.
///
/// Example:
/// ```json
/// {
///   "_key": "session",
///   "_type": "session",
///   "id": "xxx",
///   "title": "...",
///   ... // any other fields
/// }
/// ```
pub type ShareData = Value;

// Create share request
#[derive(Debug, Deserialize)]
pub struct CreateShareRequest {
    #[serde(rename = "sessionID")]
    pub session_id: String,
}

// Create share response
#[derive(Debug, Serialize)]
pub struct CreateShareResponse {
    pub id: String,
    pub secret: String,
    pub url: String,
}

// Sync share request
#[derive(Debug, Deserialize)]
pub struct SyncShareRequest {
    pub secret: String,
    pub data: Vec<ShareData>,
}

// Sync share response
#[derive(Debug, Serialize)]
pub struct SyncShareResponse {
    pub data: Vec<ShareData>,
}

// Get share response
#[derive(Debug, Serialize)]
pub struct GetShareResponse {
    pub data: Vec<ShareData>,
}

// Remove share request
#[derive(Debug, Deserialize)]
pub struct RemoveShareRequest {
    pub secret: String,
}