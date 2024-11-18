use actix_web::{test, web, App};
use mockall::predicate::*;
use mockall::mock;
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;

use crate::features::app::app_state::AppState;
use crate::features::db::{DbClient, ImageJob, ImageJobStatus};
use super::image::{
    create_image, replicate_webhook, OpenAIImageRequest, ReplicateWebhookPayload
};

// Mock external services
mock! {
    pub DbClient {
        fn create_image_job(&self, job: &ImageJob) -> Result<(), Box<dyn std::error::Error>>;
        fn update_image_job(
            &self,
            id: Uuid,
            status: ImageJobStatus,
            urls: Option<Vec<String>>,
            ipfs_urls: Option<Vec<String>>,
            error: Option<String>,
        ) -> Result<(), Box<dyn std::error::Error>>;
    }
}

#[actix_web::test]
async fn test_create_image_success() {
    // Setup mock database
    let mut mock_db = MockDbClient::new();
    mock_db.expect_create_image_job()
        .returning(|_| Ok(()));

    // Create test app state
    let app_state = AppState::new(
        "http://dify-test".to_string(),
        "replicate-key".to_string(),
        "http://ipfs-test".to_string(),
        "http://public-test".to_string(),
        "http://supabase-test".to_string(),
        "supabase-key".to_string(),
    );

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .service(web::resource("/v1/images/generations").route(web::post().to(create_image)))
    ).await;

    // Create test request
    let request = test::TestRequest::post()
        .uri("/v1/images/generations")
        .set_json(json!({
            "prompt": "test prompt",
            "size": "1024x1024",
            "model": "dall-e-3-pro",
            "user": "test-user"
        }))
        .to_request();

    // Execute request
    let response = test::call_service(&app, request).await;

    // Assert response
    assert!(response.status().is_success());
    
    let body: serde_json::Value = test::read_body_json(response).await;
    assert!(body.get("created").is_some());
    assert!(body.get("data").is_some());
}

#[actix_web::test]
async fn test_create_image_timeout() {
    // Setup mock database
    let mut mock_db = MockDbClient::new();
    mock_db.expect_create_image_job()
        .returning(|_| Ok(()));

    // Create test app state
    let app_state = AppState::new(
        "http://dify-test".to_string(),
        "replicate-key".to_string(),
        "http://ipfs-test".to_string(),
        "http://public-test".to_string(),
        "http://supabase-test".to_string(),
        "supabase-key".to_string(),
    );

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .service(web::resource("/v1/images/generations").route(web::post().to(create_image)))
    ).await;

    // Create test request with long-running task simulation
    let request = test::TestRequest::post()
        .uri("/v1/images/generations")
        .set_json(json!({
            "prompt": "test prompt",
            "size": "1024x1024",
            "model": "dall-e-3-pro",
            "user": "test-user"
        }))
        .to_request();

    // Execute request
    let response = test::call_service(&app, request).await;

    // Assert timeout response
    assert_eq!(response.status(), 408);
    
    let body: serde_json::Value = test::read_body_json(response).await;
    assert!(body.get("error").is_some());
    assert!(body["error"]["type"] == "timeout");
}

#[actix_web::test]
async fn test_webhook_success() {
    // Setup mock database
    let mut mock_db = MockDbClient::new();
    mock_db.expect_update_image_job()
        .returning(|_, _, _, _, _| Ok(()));

    // Create test app state
    let app_state = AppState::new(
        "http://dify-test".to_string(),
        "replicate-key".to_string(),
        "http://ipfs-test".to_string(),
        "http://public-test".to_string(),
        "http://supabase-test".to_string(),
        "supabase-key".to_string(),
    );

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .service(web::resource("/v1/webhook/replicate/{task_id}").route(web::post().to(replicate_webhook)))
    ).await;

    // Create test webhook payload
    let task_id = Uuid::new_v4();
    let request = test::TestRequest::post()
        .uri(&format!("/v1/webhook/replicate/{}", task_id))
        .set_json(json!({
            "id": "test-id",
            "status": "succeeded",
            "output": ["http://test-url/image1.png", "http://test-url/image2.png"]
        }))
        .to_request();

    // Execute request
    let response = test::call_service(&app, request).await;

    // Assert response
    assert!(response.status().is_success());
}

#[actix_web::test]
async fn test_webhook_failure() {
    // Setup mock database
    let mut mock_db = MockDbClient::new();
    mock_db.expect_update_image_job()
        .returning(|_, _, _, _, _| Ok(()));

    // Create test app state
    let app_state = AppState::new(
        "http://dify-test".to_string(),
        "replicate-key".to_string(),
        "http://ipfs-test".to_string(),
        "http://public-test".to_string(),
        "http://supabase-test".to_string(),
        "supabase-key".to_string(),
    );

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .service(web::resource("/v1/webhook/replicate/{task_id}").route(web::post().to(replicate_webhook)))
    ).await;

    // Create test webhook payload with failure status
    let task_id = Uuid::new_v4();
    let request = test::TestRequest::post()
        .uri(&format!("/v1/webhook/replicate/{}", task_id))
        .set_json(json!({
            "id": "test-id",
            "status": "failed",
            "output": null
        }))
        .to_request();

    // Execute request
    let response = test::call_service(&app, request).await;

    // Assert response
    assert!(response.status().is_success());
}

// Helper functions for creating test data
impl OpenAIImageRequest {
    pub fn new_test() -> Self {
        Self {
            prompt: "test prompt".to_string(),
            n: 1,
            size: "1024x1024".to_string(),
            response_format: "url".to_string(),
            model: Some("dall-e-3-pro".to_string()),
            callback_url: None,
            user: Some("test-user".to_string()),
        }
    }
}

impl ReplicateWebhookPayload {
    pub fn new_test(status: &str) -> Self {
        Self {
            id: "test-id".to_string(),
            status: status.to_string(),
            output: if status == "succeeded" {
                Some(vec!["http://test-url/image.png".to_string()])
            } else {
                None
            },
        }
    }
}
