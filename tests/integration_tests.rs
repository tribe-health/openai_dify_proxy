use actix_web::{test, web, App};
use openai_dify_proxy::features::{
    app::app_state::AppState,
    dify::handlers::openai::image::{create_image, replicate_webhook},
};
use serde_json::json;
use test_log::test;

#[test]
async fn test_full_image_generation_flow() {
    // Setup test environment
    dotenv::dotenv().ok();
    let app_state = AppState::new(
        std::env::var("DIFY_API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string()),
        std::env::var("REPLICATE_API_KEY").expect("REPLICATE_API_KEY must be set for tests"),
        std::env::var("IPFS_URL").unwrap_or_else(|_| "https://ipfs.tribemedia.io".to_string()),
        std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:8223".to_string()),
        std::env::var("SUPABASE_URL").expect("SUPABASE_URL must be set for tests"),
        std::env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set for tests"),
    );

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .service(
                web::scope("/v1")
                    .route("/images/generations", web::post().to(create_image))
                    .route("/webhook/replicate/{task_id}", web::post().to(replicate_webhook))
            )
    ).await;

    // Test image generation request
    let request = test::TestRequest::post()
        .uri("/v1/images/generations")
        .set_json(json!({
            "prompt": "test mountain landscape",
            "size": "1024x1024",
            "model": "dall-e-3-pro",
            "user": "test-user"
        }))
        .to_request();

    let response = test::call_service(&app, request).await;
    
    // Check initial response
    assert!(response.status().is_success() || response.status().as_u16() == 408);
    
    let body: serde_json::Value = test::read_body_json(response).await;
    
    if response.status().is_success() {
        // If successful immediately
        assert!(body.get("data").is_some());
        let data = body["data"].as_array().unwrap();
        assert!(!data.is_empty());
        
        // Verify image URLs
        for item in data {
            if let Some(url) = item.get("url") {
                let url_str = url.as_str().unwrap();
                let client = reqwest::Client::new();
                let img_response = client.get(url_str).send().await.unwrap();
                assert!(img_response.status().is_success());
            }
            
            if let Some(ipfs_url) = item.get("ipfs_url") {
                assert!(ipfs_url.as_str().unwrap().starts_with("ipfs://"));
            }
        }
    } else {
        // If timeout (async processing)
        assert!(body.get("error").is_some());
        assert_eq!(body["error"]["type"], "timeout");
        let task_id = body["error"]["task_id"].as_str().unwrap();
        
        // Test webhook with success response
        let webhook_request = test::TestRequest::post()
            .uri(&format!("/v1/webhook/replicate/{}", task_id))
            .set_json(json!({
                "id": "test-id",
                "status": "succeeded",
                "output": ["https://replicate.delivery/test-image.png"]
            }))
            .to_request();
            
        let webhook_response = test::call_service(&app, webhook_request).await;
        assert!(webhook_response.status().is_success());
    }
}

#[test]
async fn test_error_handling() {
    // Setup test environment with invalid credentials
    let app_state = AppState::new(
        "http://invalid-url".to_string(),
        "invalid-key".to_string(),
        "http://invalid-ipfs".to_string(),
        "http://invalid-public".to_string(),
        "http://invalid-supabase".to_string(),
        "invalid-key".to_string(),
    );

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .service(
                web::scope("/v1")
                    .route("/images/generations", web::post().to(create_image))
            )
    ).await;

    // Test with invalid request
    let request = test::TestRequest::post()
        .uri("/v1/images/generations")
        .set_json(json!({
            "prompt": "", // Empty prompt should cause error
            "size": "invalid-size",
            "model": "invalid-model"
        }))
        .to_request();

    let response = test::call_service(&app, request).await;
    assert!(response.status().is_client_error());
}

#[test]
async fn test_concurrent_requests() {
    // Setup test environment
    dotenv::dotenv().ok();
    let app_state = AppState::new(
        std::env::var("DIFY_API_URL").unwrap_or_else(|_| "http://localhost:8000".to_string()),
        std::env::var("REPLICATE_API_KEY").expect("REPLICATE_API_KEY must be set for tests"),
        std::env::var("IPFS_URL").unwrap_or_else(|_| "https://ipfs.tribemedia.io".to_string()),
        std::env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:8223".to_string()),
        std::env::var("SUPABASE_URL").expect("SUPABASE_URL must be set for tests"),
        std::env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set for tests"),
    );

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(app_state))
            .service(
                web::scope("/v1")
                    .route("/images/generations", web::post().to(create_image))
            )
    ).await;

    // Create multiple concurrent requests
    let mut handles = vec![];
    for i in 0..5 {
        let app = app.clone();
        handles.push(tokio::spawn(async move {
            let request = test::TestRequest::post()
                .uri("/v1/images/generations")
                .set_json(json!({
                    "prompt": format!("test concurrent request {}", i),
                    "size": "1024x1024",
                    "model": "dall-e-3-pro",
                    "user": format!("test-user-{}", i)
                }))
                .to_request();

            let response = test::call_service(&app, request).await;
            assert!(response.status().is_success() || response.status().as_u16() == 408);
        }));
    }

    // Wait for all requests to complete
    for handle in handles {
        handle.await.unwrap();
    }
}
