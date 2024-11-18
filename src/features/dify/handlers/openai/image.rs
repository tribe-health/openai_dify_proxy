use actix_web::{web::{Data, Json, Path}, HttpResponse};
use serde::{Deserialize, Serialize};
use crate::features::app::app_state::{AppState, ImageTaskResult};
use crate::features::db::{ImageJob, ImageJobStatus};
use uuid::Uuid;
use chrono::Utc;
use std::time::Duration;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Deserialize)]
pub struct OpenAIImageRequest {
    pub prompt: String,
    #[serde(default = "default_n")]
    pub n: u32,
    #[serde(default = "default_size")]
    pub size: String,
    #[serde(rename = "response_format")]
    #[serde(default = "default_response_format")]
    pub response_format: String,
    pub model: Option<String>,
    #[serde(default)]
    pub callback_url: Option<String>,
    pub user: Option<String>,
}

fn default_n() -> u32 { 1 }
fn default_size() -> String { "1024x1024".to_string() }
fn default_response_format() -> String { "url".to_string() }

#[derive(Debug, Serialize)]
pub struct OpenAIImageResponse {
    pub created: u64,
    pub data: Vec<ImageData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b64_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ipfs_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReplicateRequest {
    input: ReplicateInput,
    webhook: String,
}

#[derive(Debug, Serialize)]
struct ReplicateInput {
    raw: bool,
    prompt: String,
    aspect_ratio: String,
    output_format: String,
    safety_tolerance: u32,
}

#[derive(Debug, Deserialize)]
struct ReplicateResponse {
    id: String,
    status: String,
    #[serde(default)]
    output: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ReplicateWebhookPayload {
    pub id: String,
    pub status: String,
    pub output: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct IpfsAddResponse {
    #[serde(rename = "Hash")]
    hash: String,
}

fn determine_replicate_model(model: Option<String>) -> &'static str {
    match model.as_deref() {
        Some("dall-e-3-pro") => "black-forest-labs/flux-1.1-pro",
        Some("dall-e-3-pro-ultra") => "black-forest-labs/flux-1.1-pro-ultra",
        Some("dall-e-3-schnell") => "black-forest-labs/flux-1.1-schnell",
        _ => "black-forest-labs/flux-1.1-dev"
    }
}

fn convert_size_to_aspect_ratio(size: &str) -> String {
    match size {
        "1024x1024" => "1:1",
        "1024x1792" => "9:16",
        "1792x1024" => "16:9",
        _ => "3:2",
    }.to_string()
}

pub async fn create_image(
    app_state: Data<AppState>,
    request: Json<OpenAIImageRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let job_id = Uuid::new_v4();
    let replicate_model = determine_replicate_model(request.model.clone());
    
    let webhook_url = format!("{}/v1/webhook/replicate/{}", 
        app_state.public_url,
        job_id
    );

    // Create database record
    let job = ImageJob {
        id: job_id,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        status: ImageJobStatus::Processing,
        prompt: request.prompt.clone(),
        model: replicate_model.to_string(),
        size: request.size.clone(),
        urls: None,
        ipfs_urls: None,
        user_id: request.user.clone(),
        callback_url: request.callback_url.clone(),
        error: None,
    };

    app_state.db.create_image_job(&job)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let replicate_request = ReplicateRequest {
        input: ReplicateInput {
            raw: false,
            prompt: request.prompt.clone(),
            aspect_ratio: convert_size_to_aspect_ratio(&request.size),
            output_format: "png".to_string(),
            safety_tolerance: 2,
        },
        webhook: webhook_url,
    };

    // Initialize task data before making the request
    app_state.create_task(job_id.to_string(), request.callback_url.clone()).await;

    // Make request to Replicate
    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "https://api.replicate.com/v1/models/{}/predictions",
            replicate_model
        ))
        .header(
            "Authorization",
            format!("Bearer {}", app_state.replicate_api_key)
        )
        .header("Content-Type", "application/json")
        .json(&replicate_request)
        .send()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let replicate_response = response
        .json::<ReplicateResponse>()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // Wait for the result with timeout
    if let Some(result) = app_state.wait_for_task_result(&job_id.to_string(), DEFAULT_TIMEOUT).await {
        // Convert result based on requested format
        let data = if request.response_format == "b64_json" {
            convert_to_base64_response(result).await?
        } else {
            convert_to_url_response(result)
        };

        // Update database record
        app_state.db.update_image_job(
            job_id,
            ImageJobStatus::Completed,
            Some(data.iter().filter_map(|d| d.url.clone()).collect()),
            Some(data.iter().filter_map(|d| d.ipfs_url.clone()).collect()),
            None,
        )
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

        // Clean up task data
        app_state.remove_task(&job_id.to_string()).await;

        Ok(HttpResponse::Ok().json(OpenAIImageResponse {
            created: Utc::now().timestamp() as u64,
            data,
        }))
    } else {
        // Update database record with processing status
        app_state.db.update_image_job(
            job_id,
            ImageJobStatus::Processing,
            None,
            None,
            None,
        )
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

        Ok(HttpResponse::RequestTimeout().json(serde_json::json!({
            "error": {
                "message": "Request timeout. The image generation is still processing and will be sent to the callback URL when complete.",
                "type": "timeout",
                "task_id": job_id
            }
        })))
    }
}

pub async fn replicate_webhook(
    app_state: Data<AppState>,
    path: Path<String>,
    payload: Json<ReplicateWebhookPayload>,
) -> Result<HttpResponse, actix_web::Error> {
    let task_id = path.into_inner();
    let job_id = Uuid::parse_str(&task_id)
        .map_err(actix_web::error::ErrorBadRequest)?;

    if payload.status == "succeeded" {
        if let Some(urls) = payload.output.clone() {
            // Upload to IPFS
            let ipfs_urls = upload_to_ipfs(&app_state, &urls).await
                .map_err(actix_web::error::ErrorInternalServerError)?;
            
            // Set task result
            let result = ImageTaskResult {
                urls: urls.clone(),
                ipfs_urls: ipfs_urls.clone(),
            };
            app_state.set_task_result(&task_id, result.clone()).await;

            // Update database record
            app_state.db.update_image_job(
                job_id,
                ImageJobStatus::Completed,
                Some(urls),
                Some(ipfs_urls.clone()),
                None,
            )
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;

            // If there's a callback URL, notify the client
            if let Some(task_data) = app_state.get_task(&task_id).await {
                if let Some(callback_url) = task_data.callback_url {
                    let image_data: Vec<ImageData> = result.urls.into_iter()
                        .zip(ipfs_urls)
                        .map(|(url, ipfs_url)| ImageData {
                            url: Some(url),
                            b64_json: None,
                            ipfs_url: Some(ipfs_url),
                        })
                        .collect();

                    notify_client(&callback_url, &image_data).await
                        .map_err(actix_web::error::ErrorInternalServerError)?;
                }
            }
        }
    } else {
        // Update database record with error
        app_state.db.update_image_job(
            job_id,
            ImageJobStatus::Failed,
            None,
            None,
            Some(format!("Replicate job failed with status: {}", payload.status)),
        )
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    }

    Ok(HttpResponse::Ok().finish())
}

async fn convert_to_base64_response(result: ImageTaskResult) -> Result<Vec<ImageData>, actix_web::Error> {
    let client = reqwest::Client::new();
    let mut data = Vec::new();

    for url in result.urls {
        let response = client.get(&url)
            .send()
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        
        let image_bytes = response.bytes()
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        let b64_json = BASE64.encode(image_bytes);

        data.push(ImageData {
            url: None,
            b64_json: Some(b64_json),
            ipfs_url: None,
        });
    }

    Ok(data)
}

fn convert_to_url_response(result: ImageTaskResult) -> Vec<ImageData> {
    result.urls.into_iter()
        .zip(result.ipfs_urls)
        .map(|(url, ipfs_url)| ImageData {
            url: Some(url),
            b64_json: None,
            ipfs_url: Some(ipfs_url),
        })
        .collect()
}

async fn upload_to_ipfs(app_state: &AppState, urls: &[String]) -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();
    let mut ipfs_urls = Vec::new();

    for url in urls {
        // Download the image
        let response = client.get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        let image_bytes = response.bytes()
            .await
            .map_err(|e| e.to_string())?;

        // Upload to self-hosted IPFS instance
        let ipfs_client = reqwest::Client::new();
        let form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(image_bytes.to_vec()));

        let response = ipfs_client
            .post(format!("{}/api/v0/add", app_state.ipfs_url))
            .multipart(form)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let ipfs_response: IpfsAddResponse = response.json()
            .await
            .map_err(|e| e.to_string())?;

        ipfs_urls.push(format!("ipfs://{}", ipfs_response.hash));
    }

    Ok(ipfs_urls)
}

async fn notify_client(callback_url: &str, image_data: &[ImageData]) -> Result<(), String> {
    let client = reqwest::Client::new();
    client.post(callback_url)
        .json(&image_data)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
