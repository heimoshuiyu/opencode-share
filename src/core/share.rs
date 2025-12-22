use crate::models::{Share, ShareData};
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;

pub struct ShareService {
    pool: PgPool,
}

impl ShareService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, session_id: String) -> Result<Share> {
        let id = session_id.clone();
        let secret = uuid::Uuid::new_v4().to_string();

        // Check if share already exists
        let existing = sqlx::query_as::<_, Share>(
            "SELECT id, secret, session_id, data, created_at, updated_at FROM shares WHERE id = $1"
        )
        .bind(&id)
        .fetch_optional(&self.pool)
        .await?;

        if existing.is_some() {
            return Err(anyhow!("Share already exists: {}", id));
        }

        // Create new share with empty data array
        let share = sqlx::query_as::<_, Share>(
            r#"
            INSERT INTO shares (id, secret, session_id, data, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, secret, session_id, data, created_at, updated_at
            "#
        )
        .bind(&id)
        .bind(&secret)
        .bind(&session_id)
        .bind(json!([])) // Empty data array
        .bind(Utc::now())
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;

        Ok(share)
    }

    pub async fn get(&self, id: &str) -> Result<Option<Share>> {
        let share = sqlx::query_as::<_, Share>(
            "SELECT id, secret, session_id, data, created_at, updated_at FROM shares WHERE id = $1"
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

    pub async fn sync(&self, share_id: &str, secret: &str, incoming_data: Vec<ShareData>) -> Result<()> {
        let share = self.get(share_id).await?;
        let share = share.ok_or_else(|| anyhow!("Share not found: {}", share_id))?;
        
        if share.secret != secret {
            return Err(anyhow!("Share secret invalid: {}", share_id));
        }

        // Get current data
        let current_data_value = share.data.unwrap_or(json!([]));
        let mut current_data: Vec<ShareData> = if let Some(data_array) = current_data_value.as_array() {
            data_array.iter()
                .filter_map(|item| {
                    match serde_json::from_value::<ShareData>(item.clone()) {
                        Ok(data) => Some(data),
                        Err(_) => None
                    }
                })
                .collect()
        } else {
            Vec::new()
        };

        // Merge incoming data with current data
        for item in incoming_data {
            let key = self.get_data_key(&item);
            self.merge_data(&mut current_data, item, &key);
        }

        // Convert back to JSON and update database
        let updated_json = serde_json::to_value(&current_data)?;

        sqlx::query(
            r#"
            UPDATE shares 
            SET data = $2,
                updated_at = $3
            WHERE id = $1
            "#
        )
        .bind(share_id)
        .bind(updated_json)
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_data(&self, share_id: &str) -> Result<Vec<ShareData>> {
        let share = self.get(share_id).await?;
        let share = share.ok_or_else(|| anyhow!("Share not found: {}", share_id))?;

        // Simply return the stored data
        if let Some(data_value) = share.data {
            if let Some(data_array) = data_value.as_array() {
                return Ok(data_array.iter()
                    .filter_map(|item| {
                        match serde_json::from_value::<ShareData>(item.clone()) {
                            Ok(data) => Some(data),
                            Err(_) => None
                        }
                    })
                    .collect::<Vec<ShareData>>());
            }
        }

        Ok(vec![])
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