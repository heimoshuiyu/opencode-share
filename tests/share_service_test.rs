// Integration tests for ShareService

use opencode_share::core::share::ShareService;
use serde_json::json;
use sqlx::PgPool;
use std::env;

async fn get_test_pool() -> PgPool {
    // Try to get test database URL from environment
    let database_url = env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@localhost/opencode_share_test".to_string());

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

async fn setup_test_database(pool: &PgPool) {
    // Clean up any existing data
    sqlx::query("DELETE FROM shares")
        .execute(pool)
        .await
        .expect("Failed to clean test database");
}

#[tokio::test]
async fn test_create_share() {
    let pool = get_test_pool().await;
    setup_test_database(&pool).await;

    let service = ShareService::new(pool);

    // Test creating a new share
    let session_id = "test-session-1".to_string();
    let share = service
        .create(session_id.clone())
        .await
        .expect("Failed to create share");

    assert_eq!(share.id, session_id);
    assert!(!share.secret.is_empty());
    assert_eq!(share.session_id, session_id);
    assert!(share.data.is_some() || share.data.unwrap().as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_create_duplicate_share() {
    let pool = get_test_pool().await;
    setup_test_database(&pool).await;

    let service = ShareService::new(pool);

    let session_id = "test-session-duplicate".to_string();

    // Create first share
    service
        .create(session_id.clone())
        .await
        .expect("Failed to create first share");

    // Try to create duplicate share - should fail
    let result = service.create(session_id).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("already exists"));
}

#[tokio::test]
async fn test_get_share() {
    let pool = get_test_pool().await;
    setup_test_database(&pool).await;

    let service = ShareService::new(pool);

    let session_id = "test-session-get".to_string();

    // Create a share first
    let created_share = service
        .create(session_id.clone())
        .await
        .expect("Failed to create share");

    // Get the share
    let retrieved_share = service
        .get(&session_id)
        .await
        .expect("Failed to get share");

    assert!(retrieved_share.is_some());
    let share = retrieved_share.unwrap();
    assert_eq!(share.id, created_share.id);
    assert_eq!(share.secret, created_share.secret);
    assert_eq!(share.session_id, created_share.session_id);
}

#[tokio::test]
async fn test_get_nonexistent_share() {
    let pool = get_test_pool().await;
    setup_test_database(&pool).await;

    let service = ShareService::new(pool);

    // Try to get a share that doesn't exist
    let result = service.get("nonexistent-id").await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_remove_share() {
    let pool = get_test_pool().await;
    setup_test_database(&pool).await;

    let service = ShareService::new(pool);

    let session_id = "test-session-remove".to_string();

    // Create a share first
    let created_share = service
        .create(session_id.clone())
        .await
        .expect("Failed to create share");

    // Remove the share with correct secret
    service
        .remove(&session_id, &created_share.secret)
        .await
        .expect("Failed to remove share");

    // Verify share is removed
    let result = service.get(&session_id).await;
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn test_remove_share_with_invalid_secret() {
    let pool = get_test_pool().await;
    setup_test_database(&pool).await;

    let service = ShareService::new(pool);

    let session_id = "test-session-invalid-secret".to_string();

    // Create a share first
    service
        .create(session_id.clone())
        .await
        .expect("Failed to create share");

    // Try to remove with invalid secret
    let result = service.remove(&session_id, "invalid-secret").await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("secret invalid"));

    // Verify share still exists
    let retrieved = service.get(&session_id).await;
    assert!(retrieved.unwrap().is_some());
}

#[tokio::test]
async fn test_remove_nonexistent_share() {
    let pool = get_test_pool().await;
    setup_test_database(&pool).await;

    let service = ShareService::new(pool);

    // Try to remove a share that doesn't exist
    let result = service.remove("nonexistent-id", "some-secret").await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn test_sync_share_data() {
    let pool = get_test_pool().await;
    setup_test_database(&pool).await;

    let service = ShareService::new(pool);

    let session_id = "test-session-sync".to_string();

    // Create a share first
    let created_share = service
        .create(session_id.clone())
        .await
        .expect("Failed to create share");

    // Create test data to sync (now using arbitrary JSON with _key field)
    let test_data = vec![
        json!({
            "_key": "session",
            "model": "gpt-4",
            "messages": []
        }),
        json!({
            "_key": "message/msg-1",
            "id": "msg-1",
            "role": "user",
            "content": "Hello"
        }),
    ];

    // Sync data to share
    service
        .sync(&session_id, &created_share.secret, test_data.clone())
        .await
        .expect("Failed to sync data");

    // Retrieve and verify the data
    let retrieved_data = service
        .get_data(&session_id)
        .await
        .expect("Failed to get data");

    assert_eq!(retrieved_data.len(), 2);
}

#[tokio::test]
async fn test_sync_with_invalid_secret() {
    let pool = get_test_pool().await;
    setup_test_database(&pool).await;

    let service = ShareService::new(pool);

    let session_id = "test-session-sync-invalid".to_string();

    // Create a share first
    service
        .create(session_id.clone())
        .await
        .expect("Failed to create share");

    // Try to sync with invalid secret
    let test_data = vec![json!({
        "_key": "session",
        "model": "gpt-4"
    })];

    let result = service.sync(&session_id, "invalid-secret", test_data).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("secret invalid"));
}

#[tokio::test]
async fn test_get_share_data() {
    let pool = get_test_pool().await;
    setup_test_database(&pool).await;

    let service = ShareService::new(pool);

    let session_id = "test-session-get-data".to_string();

    // Create a share first
    let created_share = service
        .create(session_id.clone())
        .await
        .expect("Failed to create share");

    // Sync some data
    let test_data = vec![
        json!({
            "_key": "session",
            "model": "gpt-4",
            "messages": []
        }),
        json!({
            "_key": "message/msg-1",
            "id": "msg-1",
            "role": "user",
            "content": "Test message"
        }),
    ];

    service
        .sync(&session_id, &created_share.secret, test_data)
        .await
        .expect("Failed to sync data");

    // Get the data
    let retrieved_data = service
        .get_data(&session_id)
        .await
        .expect("Failed to get data");

    assert_eq!(retrieved_data.len(), 2);

    // Verify the data structure
    let session_data = &retrieved_data[0];
    assert_eq!(session_data.get("_key").and_then(|v| v.as_str()), Some("session"));
    assert_eq!(session_data.get("model").and_then(|v| v.as_str()), Some("gpt-4"));

    let message_data = &retrieved_data[1];
    assert_eq!(message_data.get("_key").and_then(|v| v.as_str()), Some("message/msg-1"));
    assert_eq!(message_data.get("id").and_then(|v| v.as_str()), Some("msg-1"));
    assert_eq!(message_data.get("content").and_then(|v| v.as_str()), Some("Test message"));
}

#[tokio::test]
async fn test_merge_data_same_key() {
    let pool = get_test_pool().await;
    setup_test_database(&pool).await;

    let service = ShareService::new(pool);

    let session_id = "test-session-merge".to_string();

    // Create a share first
    let created_share = service
        .create(session_id.clone())
        .await
        .expect("Failed to create share");

    // Sync initial data
    let initial_data = vec![json!({
        "_key": "session",
        "model": "gpt-3.5",
        "messages": []
    })];

    service
        .sync(&session_id, &created_share.secret, initial_data)
        .await
        .expect("Failed to sync initial data");

    // Sync updated data with same key
    let updated_data = vec![json!({
        "_key": "session",
        "model": "gpt-4",
        "messages": [],
        "temperature": 0.7
    })];

    service
        .sync(&session_id, &created_share.secret, updated_data)
        .await
        .expect("Failed to sync updated data");

    // Get the data and verify it was updated (not duplicated)
    let retrieved_data = service
        .get_data(&session_id)
        .await
        .expect("Failed to get data");

    assert_eq!(retrieved_data.len(), 1); // Should still be only 1 session

    let session_data = &retrieved_data[0];
    assert_eq!(session_data.get("_key").and_then(|v| v.as_str()), Some("session"));
    assert_eq!(session_data.get("model").and_then(|v| v.as_str()), Some("gpt-4"));
    assert_eq!(session_data.get("temperature").and_then(|v| v.as_f64()), Some(0.7));
}

#[tokio::test]
async fn test_get_empty_share_data() {
    let pool = get_test_pool().await;
    setup_test_database(&pool).await;

    let service = ShareService::new(pool);

    let session_id = "test-session-empty-data".to_string();

    // Create a share without syncing any data
    service
        .create(session_id.clone())
        .await
        .expect("Failed to create share");

    // Get the data - should be empty
    let retrieved_data = service
        .get_data(&session_id)
        .await
        .expect("Failed to get data");

    assert!(retrieved_data.is_empty());
}
