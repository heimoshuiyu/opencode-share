use crate::models::{Share, ShareData};
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::error;
use uuid::Uuid;

pub struct ShareService {
    pool: PgPool,
}

impl ShareService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, session_id: String) -> Result<Share> {
        let id = session_id.clone();
        let secret = Uuid::new_v4().to_string();

        // Check if share already exists
        let existing = sqlx::query_as::<_, Share>(
            "SELECT id, secret, session_id, events, compacted_data, created_at, updated_at FROM shares WHERE id = $1"
        )
        .bind(&id)
        .fetch_optional(&self.pool)
        .await?;

        if existing.is_some() {
            return Err(anyhow!("Share already exists: {}", id));
        }

        // Create new share with empty events array
        let share = sqlx::query_as::<_, Share>(
            r#"
            INSERT INTO shares (id, secret, session_id, events, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, secret, session_id, events, compacted_data, created_at, updated_at
            "#
        )
        .bind(&id)
        .bind(&secret)
        .bind(&session_id)
        .bind(json!([])) // Empty events array
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;

        Ok(share)
    }

    pub async fn get(&self, id: &str) -> Result<Option<Share>> {
        let share = sqlx::query_as::<_, Share>(
            "SELECT id, secret, session_id, events, compacted_data, created_at, updated_at FROM shares WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(share)
    }

    pub async fn remove(&self, id: &str, secret: &str) -> Result<()> {
        let share = self.get(id).await?;
        let share = share.ok_or_else(|| anyhow!("Share not found: {}", id))?;
        
        if share.secret != secret {
            return Err(anyhow!("Share secret invalid: {}", id));
        }

        // Delete share (single table operation)
        sqlx::query("DELETE FROM shares WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn sync(&self, share_id: &str, secret: &str, data: Vec<ShareData>) -> Result<()> {
        let share = self.get(share_id).await?;
        let share = share.ok_or_else(|| anyhow!("Share not found: {}", share_id))?;
        
        if share.secret != secret {
            return Err(anyhow!("Share secret invalid: {}", share_id));
        }

        // Convert ShareData to ShareEvent
        let new_events: Vec<Value> = data.into_iter().map(|share_data| {
            let event_key = format!("event_{}", Uuid::new_v4());
            json!({
                "event_key": event_key,
                "type": match &share_data {
                    ShareData::Session { .. } => "session",
                    ShareData::Message { .. } => "message",
                    ShareData::Part { .. } => "part",
                    ShareData::SessionDiff { .. } => "session_diff",
                    ShareData::Model { .. } => "model",
                },
                "data": match share_data {
                    ShareData::Session { data } => data,
                    ShareData::Message { data } => data,
                    ShareData::Part { data } => data,
                    ShareData::SessionDiff { data } => data,
                    ShareData::Model { data } => data,
                },
                "created_at": Utc::now().to_rfc3339()
            })
        }).collect();

        // Append new events to existing events array
        sqlx::query(
            r#"
            UPDATE shares 
            SET events = events || $2::jsonb,
                updated_at = $3
            WHERE id = $1
            "#
        )
        .bind(share_id)
        .bind(Value::Array(new_events))
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_data(&self, share_id: &str) -> Result<Vec<ShareData>> {
        let share = self.get(share_id).await?;
        let share = share.ok_or_else(|| anyhow!("Share not found: {}", share_id))?;

        // Try to get compacted data first (if available)
        if let Some(compact_data) = share.compacted_data {
            if let Some(data_array) = compact_data.as_array() {
                let mut result = Vec::new();
                for item in data_array {
                    if let Ok(share_data) = serde_json::from_value::<ShareData>(item.clone()) {
                        result.push(share_data);
                    }
                }
                return Ok(result);
            }
        }

        // Fallback to processing events
        let events_value = share.events.unwrap_or(json!([]));
        
        if let Some(events) = events_value.as_array() {
            let mut result = Vec::new();
            
            for event in events {
                // Extract ShareData from event
                let share_data: Result<ShareData, String> = if let Some(event_type) = event.get("type").and_then(|v| v.as_str()) {
                    match event_type {
                        "session" => Ok(ShareData::Session { 
                            data: event.get("data").cloned().unwrap_or(json!({})) 
                        }),
                        "message" => Ok(ShareData::Message { 
                            data: event.get("data").cloned().unwrap_or(json!({})) 
                        }),
                        "part" => Ok(ShareData::Part { 
                            data: event.get("data").cloned().unwrap_or(json!({})) 
                        }),
                        "session_diff" => Ok(ShareData::SessionDiff { 
                            data: event.get("data").cloned().unwrap_or(json!({})) 
                        }),
                        "model" => Ok(ShareData::Model { 
                            data: event.get("data").cloned().unwrap_or(json!({})) 
                        }),
                        _ => {
                            error!("Unknown event type: {}", event_type);
                            Err(format!("Unknown event type: {}", event_type))
                        }
                    }
                } else {
                    error!("Event missing type field");
                    Err("Event missing type field".to_string())
                };

                match share_data {
                    Ok(data) => {
                        let key = self.get_data_key(&data);
                        self.merge_data(&mut result, data, &key);
                    }
                    Err(e) => {
                        error!("Failed to parse event data: {}", e);
                        continue;
                    }
                }
            }

            // Optional: Update compaction if we have enough events
            if result.len() > 10 {
                if let Err(e) = self.update_compaction(share_id, &result).await {
                    error!("Failed to update compaction: {}", e);
                }
            }

            Ok(result)
        } else {
            Ok(vec![])
        }
    }

    async fn update_compaction(&self, share_id: &str, data: &[ShareData]) -> Result<()> {
        let compacted_json = serde_json::to_value(data)?;
        
        sqlx::query(
            r#"
            UPDATE shares 
            SET compacted_data = $2,
                updated_at = $3
            WHERE id = $1
            "#
        )
        .bind(share_id)
        .bind(compacted_json)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    fn get_data_key(&self, data: &ShareData) -> String {
        match data {
            ShareData::Session { .. } => "session".to_string(),
            ShareData::Message { data } => {
                if let Some(msg_id) = data.get("id").and_then(|v| v.as_str()) {
                    format!("message/{}", msg_id)
                } else {
                    "message/unknown".to_string()
                }
            }
            ShareData::Part { data } => {
                let msg_id = data.get("messageID").and_then(|v| v.as_str()).unwrap_or("unknown");
                let part_id = data.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                format!("{}/{}", msg_id, part_id)
            }
            ShareData::SessionDiff { .. } => "session_diff".to_string(),
            ShareData::Model { .. } => "model".to_string(),
        }
    }

    fn merge_data(&self, result: &mut Vec<ShareData>, item: ShareData, key: &str) {
        // Simple linear search and replace/insert
        if let Some(index) = result.iter().position(|existing| self.get_data_key(existing) == key) {
            result[index] = item;
        } else {
            result.push(item);
        }
    }
}