use crate::models::{Share, ShareData, ShareEvent, ShareCompaction};
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde_json;
use sqlx::SqlitePool;
use tracing::error;
use uuid::Uuid;

pub struct ShareService {
    pool: SqlitePool,
}

impl ShareService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, session_id: String) -> Result<Share> {
        let is_test = session_id.starts_with("test_");
        let id = if is_test {
            format!("test_{}", &session_id[session_id.len().saturating_sub(8)..])
        } else {
            session_id[session_id.len().saturating_sub(8)..].to_string()
        };
        
        let secret = Uuid::new_v4().to_string();

        // Check if share already exists
        let existing = sqlx::query_as::<_, Share>(
            "SELECT id, secret, session_id, created_at FROM shares WHERE id = ?"
        )
        .bind(&id)
        .fetch_optional(&self.pool)
        .await?;

        if existing.is_some() {
            return Err(anyhow!("Share already exists: {}", id));
        }

        // Create new share
        let share = sqlx::query_as::<_, Share>(
            r#"
            INSERT INTO shares (id, secret, session_id, created_at)
            VALUES (?, ?, ?, ?)
            RETURNING id, secret, session_id, created_at
            "#
        )
        .bind(&id)
        .bind(&secret)
        .bind(&session_id)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;

        Ok(share)
    }

    pub async fn get(&self, id: &str) -> Result<Option<Share>> {
        let share = sqlx::query_as::<_, Share>(
            "SELECT id, secret, session_id, created_at FROM shares WHERE id = ?"
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

        // Delete share (cascades to events and compactions)
        sqlx::query("DELETE FROM shares WHERE id = ?")
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

        // Generate event key for ordering
        let event_key = format!("event_{}", Uuid::new_v4());
        
        // Insert event data
        let data_json = serde_json::to_string(&data)?;
        
        sqlx::query(
            r#"
            INSERT INTO share_events (share_id, event_key, data, created_at)
            VALUES (?, ?, ?, ?)
            "#
        )
        .bind(share_id)
        .bind(&event_key)
        .bind(data_json)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_data(&self, share_id: &str) -> Result<Vec<ShareData>> {
        // Get current compaction
        let compaction = sqlx::query_as::<_, ShareCompaction>(
            "SELECT share_id, event_key, data, updated_at FROM share_compactions WHERE share_id = ?"
        )
        .bind(share_id)
        .fetch_optional(&self.pool)
        .await?;

        let mut result = match &compaction {
            Some(comp) => {
                serde_json::from_str::<Vec<ShareData>>(&comp.data).unwrap_or_else(|e| {
                    error!("Failed to parse compaction data: {}", e);
                    vec![]
                })
            }
            None => vec![],
        };

        // Get pending events
        let last_event_key = compaction.as_ref().and_then(|c| c.event_key.clone());
        
        let events = if let Some(ref key) = last_event_key {
            sqlx::query_as::<_, ShareEvent>(
                "SELECT id, share_id, event_key, data, created_at FROM share_events WHERE share_id = ? AND event_key > ? ORDER BY event_key ASC"
            )
            .bind(share_id)
            .bind(key)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, ShareEvent>(
                "SELECT id, share_id, event_key, data, created_at FROM share_events WHERE share_id = ? ORDER BY event_key ASC"
            )
            .bind(share_id)
            .fetch_all(&self.pool)
            .await?
        };

        if !events.is_empty() {
            // Process events and update result
            for event in &events {
                let event_data: Vec<ShareData> = serde_json::from_str(&event.data)
                    .unwrap_or_else(|e| {
                        error!("Failed to parse event data: {}", e);
                        vec![]
                    });

                // Merge event data with result (similar to binary search and replace logic)
                for item in event_data {
                    let key = self.get_data_key(&item);
                    self.merge_data(&mut result, item, &key);
                }
            }

            // Update compaction
            let compaction_data = serde_json::to_string(&result)?;
            let last_event_key = events.last().map(|e| e.event_key.clone());

            sqlx::query(
                r#"
                INSERT INTO share_compactions (share_id, event_key, data, updated_at)
                VALUES (?, ?, ?, ?)
                ON CONFLICT(share_id) DO UPDATE SET
                    event_key = excluded.event_key,
                    data = excluded.data,
                    updated_at = excluded.updated_at
                "#
            )
            .bind(share_id)
            .bind(last_event_key)
            .bind(compaction_data)
            .bind(Utc::now())
            .execute(&self.pool)
            .await?;
        }

        Ok(result)
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
        // In a production system, you might want a more efficient approach
        if let Some(index) = result.iter().position(|existing| self.get_data_key(existing) == key) {
            result[index] = item;
        } else {
            result.push(item);
        }
    }
}