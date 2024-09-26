use actix_web::{web, HttpResponse, Error, HttpRequest};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::{env, time::Duration, path::PathBuf};
use tokio::{time::sleep, fs::File as TokioFile};
use uuid::Uuid;
use image::load_from_memory;
use image::ImageFormat;
use image::ExtendedColorType;
use reqwest::multipart::{Part, Form};
use tokio::fs::File;
use std::io::Cursor;
use image::ImageEncoder;

#[derive(Deserialize, Serialize)]
pub struct ImageGenerationRequest {
    prompt: String,
    n: Option<u8>,
    size: Option<String>,
    return_ipfs: Option<bool>,
}

#[derive(Serialize)]
pub struct ImageGenerationResponse {
    url: String,
}

#[derive(Deserialize, Serialize)]
struct ReplicateRequest {
    version: String,
    input: ReplicateInput,
}

#[derive(Deserialize, Serialize)]
struct ReplicateInput {
    prompt: String,
    num_outputs: u8,
    width: u32,
    height: u32,
    // Add other Replicate-specific parameters here
    #[serde(skip_serializing_if = "Option::is_none")]
    scheduler: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_inference_steps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guidance_scale: Option<f32>,
    output_format: Option<String>,
    output_quality: Option<u8>,
}

#[derive(Deserialize, Serialize)]
struct ReplicatePrediction {
    id: String,
    status: String,
    output: Option<Vec<String>>,
}

pub async fn generate_image(
    req: HttpRequest,
    payload: web::Json<ImageGenerationRequest>,
) -> Result<HttpResponse, Error> {
    let api_key = env::var("REPLICATE_API_KEY").expect("REPLICATE_API_KEY must be set");
    let image_dir = env::var("IMAGE_STORAGE_DIR").expect("IMAGE_STORAGE_DIR must be set");
    let ipfs_upload_url = env::var("IPFS_UPLOAD_URL").expect("IPFS_UPLOAD_URL must be set");
    let replicate_api_url = env::var("REPLICATE_API_URL").expect("REPLICATE_API_URL must be set");
    let client = Client::new();

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
    let response = client.post(format!("{}", replicate_api_url))
        .header("Authorization", format!("Token {}", api_key))
        .json(&replicate_request)
        .send()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let mut prediction: ReplicatePrediction = response.json()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // Poll for completion
    for _ in 0..60 { // Maximum 60 attempts (5 minutes)
        if prediction.status == "succeeded" {
            break;
        } else if prediction.status == "failed" {
            return Err(actix_web::error::ErrorInternalServerError("Image generation failed"));
        }

        sleep(Duration::from_secs(5)).await;

        let response = client.get(format!("{}/predictions/{}", replicate_api_url, prediction.id))
            .header("Authorization", format!("Token {}", api_key))
            .send()
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        prediction = response.json()
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    }

    if prediction.status != "succeeded" {
        return Err(actix_web::error::ErrorInternalServerError("Timeout waiting for image generation"));
    }

    let url = prediction.output
        .and_then(|arr| arr.first().cloned())
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("No image URL returned"))?;

    // Download the image
    let image_bytes = client.get(&url)
        .send()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?
        .bytes()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // Process the image based on the requested output format

    let (processed_image, extension, mime_type) = process_image(
        &image_bytes,
        &output_format,
        &image_dir
    ).await?;

    // Convert processed_image to Vec<u8>
    let processed_image_vec = processed_image.to_vec().clone();
    let processed_image_vec1 = processed_image_vec.clone();

    // Generate a unique filename
    let file_name = format!("{}.{}", Uuid::new_v4(), extension);
    let file_path = PathBuf::from(&image_dir).join(&file_name);

    // Save the image locally
    let mut file = TokioFile::create(&file_path)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    tokio::io::AsyncWriteExt::write_all(&mut file, &processed_image_vec)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    // Upload to IPFS
    let form = Form::new()
        .part("file", Part::bytes(processed_image_vec).file_name(file_name).mime_str(&mime_type).unwrap());

    let ipfs_response = client.post(&ipfs_upload_url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let ipfs_json: serde_json::Value = ipfs_response.json()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let ipfs_url = ipfs_json["Hash"].as_str()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Failed to get IPFS hash"))?;

    // Determine the response based on the return_ipfs parameter
    if payload.return_ipfs.unwrap_or(false) {
        Ok(HttpResponse::Ok().json(ImageGenerationResponse { url: format!("ipfs://{}", ipfs_url) }))
    } else {
        Ok(HttpResponse::Ok()
            .content_type(mime_type)
            .body(processed_image_vec1.clone()))
    }
}

async fn process_image(image_bytes: &[u8], output_format: &Option<String>, image_dir: &str) -> Result<(Vec<u8>, String, String), Error> {
    let img = load_from_memory(image_bytes)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let format = output_format.as_deref().unwrap_or("png");
    match format {
        "png" => {
            let img_clone = img.clone();
            let output_file_path = format!("{}/{}.png", image_dir, Uuid::new_v4());
            let mut file = File::create(&output_file_path)
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &img.into_bytes())
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
            Ok((img_clone.into_bytes(), "png".to_string(), "image/png".to_string()))
        },
        "webp" => {
            let rgba = img.to_rgba8();
            let mut output = Vec::new();
            image::codecs::webp::WebPEncoder::new_lossless(&mut output)
                .write_image(&rgba, rgba.width(), rgba.height(), ExtendedColorType::Rgba8)
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
            Ok((img.clone().into_bytes(), "webp".to_string(), "image/webp".to_string()))
        },
        _ => { // Default to JPEG
            let mut buffer = Vec::new();
            let mut cursor = Cursor::new(&mut buffer);
            img.write_to(&mut cursor, ImageFormat::Jpeg)
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
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
