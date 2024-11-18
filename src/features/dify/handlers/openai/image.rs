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

// Rest of the file remains the same...
