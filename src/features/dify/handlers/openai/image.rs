use actix_web::{web, HttpResponse, Error, HttpRequest};
use serde::{Deserialize, Serialize};
use awc::Client;
use std::{env, time::Duration};
use tokio::time::sleep;
use uuid::Uuid;
use image::load_from_memory;
use image::ImageFormat;
use image::ExtendedColorType;
use tokio::fs::File;
use std::io::Cursor;
use image::ImageEncoder;
use log::{error, info};
use serde_json::Value;
use actix_web::http::StatusCode;
use anyhow::Result;
use mime::Mime;
use actix_multipart::Multipart;

pub async fn generate_image(
    req: HttpRequest,
    payload: web::Json<ImageGenerationRequest>,
) -> Result<HttpResponse, Error> {
    let api_key = env::var("REPLICATE_API_KEY").map_err(|e| {
        error!("Failed to retrieve REPLICATE_API_KEY: {}", e);
        actix_web::error::ErrorInternalServerError("Failed to retrieve REPLICATE_API_KEY")
    })?;
    let image_dir = env::var("IMAGE_STORAGE_DIR").map_err(|e| {
        error!("Failed to retrieve IMAGE_STORAGE_DIR: {}", e);
        actix_web::error::ErrorInternalServerError("Failed to retrieve IMAGE_STORAGE_DIR")
    })?;
    let ipfs_upload_url = env::var("IPFS_UPLOAD_URL").map_err(|e| {
        error!("Failed to retrieve IPFS_UPLOAD_URL: {}", e);
        actix_web::error::ErrorInternalServerError("Failed to retrieve IPFS_UPLOAD_URL")
    })?;
    let replicate_api_url = env::var("REPLICATE_API_URL").map_err(|e| {
        error!("Failed to retrieve REPLICATE_API_URL: {}", e);
        actix_web::error::ErrorInternalServerError("Failed to retrieve REPLICATE_API_URL")
    })?;
    let client = Client::default();

    let (width, height) = parse_size(payload.size.as_deref().unwrap_or("1024x1024"));

    let mut replicate_input = ReplicateInput {
        prompt: payload.prompt.clone(),
        num_outputs: payload.n.unwrap_or(1),
        width,
        height,
        scheduler: None,
        num_inference_steps: None,
        guidance_scale: None,
        output_format: Some("png".to_string()),
        output_quality: None,
    };

    // Parse additional parameters from headers
    if let Some(scheduler) = req.headers().get("X-Replicate-Scheduler") {
        replicate_input.scheduler = Some(scheduler.to_str().unwrap().to_string());
    }
    if let Some(steps) = req.headers().get("X-Replicate-Num-Inference-Steps") {
        replicate_input.num_inference_steps = steps.to_str().unwrap().parse().ok();
    }
    if let Some(guidance) = req.headers().get("X-Replicate-Guidance-Scale") {
        replicate_input.guidance_scale = guidance.to_str().unwrap().parse().ok();
    }

    if let Some(output_format) = req.headers().get("X-Output-Format") {
        replicate_input.output_format = output_format.to_str().unwrap().parse().ok();
    }
    if let Some(output_quality) = req.headers().get("X-Output-Quality") {
        replicate_input.output_quality = output_quality.to_str().unwrap().parse().ok();
    }
    // ... add more header parsing for other parameters
    let output_format = replicate_input.output_format.clone();

    let replicate_request = ReplicateRequest {
        version: "flux-1-pro".to_string(),
        input: replicate_input,
    };

    // Start the prediction
    let mut response = client.post(format!("{}", replicate_api_url))
        .insert_header(("Authorization", format!("Bearer {}", api_key)))
        .send_json(&replicate_request)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let _body: Value = response.json().await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    match response.status() {
        StatusCode::OK => {
            let mut prediction = response.json::<ReplicatePrediction>()
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
            
            for _ in 0..60 {
                if prediction.status == "succeeded" {
                    break;
                } else if prediction.status == "failed" {
                    return Err(actix_web::error::ErrorInternalServerError("Image generation failed"));
                }
            
                sleep(Duration::from_secs(5)).await;
            
                let mut prediction_response = client.get(format!("{}/predictions/{}", replicate_api_url, prediction.id))
                    .bearer_auth(&api_key)
                    .send()
                    .await
                    .map_err(|e| {
                        error!("Failed to check image generation status: {}", e);
                        actix_web::error::ErrorInternalServerError("Failed to check image generation status")
                    })?;
            
                prediction = prediction_response.json::<ReplicatePrediction>()
                    .await
                    .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
            
                if prediction.status != "succeeded" {
                    continue;
                }
            
                let url = prediction.output
                    .and_then(|arr| arr.first().cloned())
                    .ok_or_else(|| actix_web::error::ErrorInternalServerError("No image URL returned"))?;
            
                // Download the image
                let image_data = client.get(&url)
                    .send()
                    .await
                    .map_err(|e| {
                        error!("Failed to download image: {}", e);
                        actix_web::error::ErrorInternalServerError("Failed to download image")
                    })?
                    .body()
                    .await
                    .map_err(|e| {
                        error!("Failed to read image bytes: {}", e);
                        actix_web::error::ErrorInternalServerError("Failed to read image bytes")
                    })?;
            
                // Upload the image to IPFS
                let _ipfs_hash = upload_to_ipfs(image_data.to_vec()).await
                    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
            
                // Process the image based on the requested output format
                let (processed_image, extension, mime_type) = process_image(
                    &image_data,
                    &output_format,
                    &image_dir
                ).await?;
            
                info!("Image generated successfully");
                return Ok(HttpResponse::Ok()
                    .content_type(mime_type)
                    .body(processed_image));
            }
            
            Err(actix_web::error::ErrorInternalServerError("Timeout waiting for image generation"))
            
        },
        _ => {
            return Err(actix_web::error::ErrorInternalServerError("Failed to start image generation"));
        }
    }
}

async fn process_image(image_bytes: &[u8], output_format: &Option<String>, image_dir: &str) -> Result<(Vec<u8>, String, String), Error> {
    let img = load_from_memory(image_bytes)
        .map_err(|e| {
            error!("Failed to load image: {}", e);
            actix_web::error::ErrorInternalServerError("Failed to load image")
        })?;

    let format = output_format.as_deref().unwrap_or("png");
    match format {
        "png" => {
            let img_clone = img.clone();
            let output_file_path = format!("{}/{}.png", image_dir, Uuid::new_v4());
            let mut file = File::create(&output_file_path)
                .await
                .map_err(|e| {
                    error!("Failed to create output file: {}", e);
                    actix_web::error::ErrorInternalServerError("Failed to create output file")
                })?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &img.into_bytes())
                .await
                .map_err(|e| {
                    error!("Failed to write to output file: {}", e);
                    actix_web::error::ErrorInternalServerError("Failed to write to output file")
                })?;
            Ok((img_clone.into_bytes(), "png".to_string(), "image/png".to_string()))
        },
        "webp" => {
            let rgba = img.to_rgba8();
            let mut output = Vec::new();
            image::codecs::webp::WebPEncoder::new_lossless(&mut output)
                .write_image(&rgba, rgba.width(), rgba.height(), ExtendedColorType::Rgba8)
                .map_err(|e| {
                    error!("Failed to encode image as WebP: {}", e);
                    actix_web::error::ErrorInternalServerError("Failed to encode image as WebP")
                })?;
            Ok((output, "webp".to_string(), "image/webp".to_string()))
        },
        _ => { // Default to JPEG
            let mut buffer = Vec::new();
            let mut cursor = Cursor::new(&mut buffer);
            img.write_to(&mut cursor, ImageFormat::Jpeg)
                .map_err(|e| {
                    error!("Failed to encode image as JPEG: {}", e);
                    actix_web::error::ErrorInternalServerError("Failed to encode image as JPEG")
                })?;
            Ok((buffer, "jpg".to_string(), "image/jpeg".to_string()))
        }
    }
}

fn parse_size(size: &str) -> (u32, u32) {
    let parts: Vec<&str> = size.split('x').collect();
    if parts.len() == 2 {
        let width = parts[0].parse().unwrap_or(1024);
        let height = parts[1].parse().unwrap_or(1024);
        (width, height)
    } else {
        (1024, 1024)
    }
}

async fn upload_to_ipfs(image_data: Vec<u8>) -> Result<String, Box<dyn std::error::Error>> {
    let ipfs_upload_url = env::var("IPFS_UPLOAD_URL").expect("IPFS_UPLOAD_URL must be set");
    let client = Client::default();

    let form = awc::multipart::Form::new()
        .part("file", awc::multipart::Part::stream(image_data)
            .content_type(mime::IMAGE_PNG)
            .file_name("image.png"));

    let mut response = client.post(&ipfs_upload_url)
        .content_type("multipart/form-data")
        .send_form(form)
        .await?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;
        Ok(result["Hash"].as_str().unwrap_or("").to_string())
    } else {
        Err("Failed to upload to IPFS".into())
    }
}
