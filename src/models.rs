use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Share {
    pub id: String,
    pub secret: String,
    pub session_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ShareData {
    #[serde(rename = "session")]
    Session { data: serde_json::Value },
    #[serde(rename = "message")]
    Message { data: serde_json::Value },
    #[serde(rename = "part")]
    Part { data: serde_json::Value },
    #[serde(rename = "session_diff")]
    SessionDiff { data: serde_json::Value },
    #[serde(rename = "model")]
    Model { data: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ShareEvent {
    pub id: i64,
    pub share_id: String,
    pub event_key: String,
    pub data: String, // JSON string
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ShareCompaction {
    pub share_id: String,
    pub event_key: Option<String>,
    pub data: String, // JSON string
    pub updated_at: DateTime<Utc>,
}

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

// Remove share request
#[derive(Debug, Deserialize)]
pub struct RemoveShareRequest {
    pub secret: String,
}