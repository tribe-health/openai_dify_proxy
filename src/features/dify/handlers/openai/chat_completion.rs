use actix_web::{web, HttpRequest, HttpResponse};
use reqwest::{Client, StatusCode};
use serde_json::json;
use lazy_static::lazy_static;
use std::env;
use std::time::Duration;
use futures_util::{StreamExt, Stream};
use bytes::Bytes;
use std::pin::Pin;
use log::debug;

use crate::features::common::types::{OpenAIRequest, DifyRequest, OpenAIResponse, OpenAIChoice, OpenAIDelta, DifyEvent};

#[derive(Debug)]
pub enum ChatCompletionError {
    InvalidApiKey,
    RequestConstructionError(String),
    DifyApiError(StatusCode, String),
    JsonSerializationError(String),
}

impl actix_web::ResponseError for ChatCompletionError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            ChatCompletionError::InvalidApiKey => actix_web::http::StatusCode::UNAUTHORIZED,
            ChatCompletionError::RequestConstructionError(_) => actix_web::http::StatusCode::BAD_REQUEST,
            ChatCompletionError::DifyApiError(status, _) => {
                reqwest_to_actix_status(*status)
            },
            ChatCompletionError::JsonSerializationError(_) => actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let message = match self {
            ChatCompletionError::InvalidApiKey => "Missing or invalid Authorization header",
            ChatCompletionError::RequestConstructionError(msg) => msg,
            ChatCompletionError::DifyApiError(_, msg) => msg,
            ChatCompletionError::JsonSerializationError(msg) => msg,
        };
        HttpResponse::build(self.status_code())
            .json(create_error_response(message))
    }
}

impl std::fmt::Display for ChatCompletionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidApiKey => write!(f, "Missing or invalid Authorization header"),
            Self::RequestConstructionError(msg) => write!(f, "Request construction error: {}", msg),
            Self::DifyApiError(status, msg) => write!(f, "Dify API error ({}): {}", status, msg),
            Self::JsonSerializationError(msg) => write!(f, "JSON serialization error: {}", msg),
        }
    }
}

fn reqwest_to_actix_status(status: StatusCode) -> actix_web::http::StatusCode {
    actix_web::http::StatusCode::from_u16(status.as_u16())
        .unwrap_or(actix_web::http::StatusCode::BAD_GATEWAY)
}

fn create_error_response(message: &str) -> serde_json::Value {
    json!({
        "error": {
            "message": message,
            "type": "invalid_request_error"
        }
    })
}

fn extract_api_key(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_str| {
            if auth_str.starts_with("Bearer ") {
                Some(auth_str[7..].to_string())
            } else {
                None
            }
        })
}

async fn construct_dify_request(
    openai_req: &OpenAIRequest,
    user: Option<String>,
) -> Result<DifyRequest, ChatCompletionError> {
    // Convert messages to query format
    let query = openai_req.messages.iter()
        .map(|msg| {
            match &msg.content {
                crate::features::common::types::MessageContent::String(s) => s.clone(),
                crate::features::common::types::MessageContent::Complex(contents) => {
                    contents.iter()
                        .map(|c| c.text.clone())
                        .collect::<Vec<String>>()
                        .join(" ")
                }
            }
        })
        .collect();

    let dify_request = DifyRequest {
        inputs: json!({}),
        query,
        response_mode: if openai_req.stream.unwrap_or(false) {
            "streaming".to_string()
        } else {
            "blocking".to_string()
        },
        user: user.unwrap_or_else(|| "default_user".to_string()),
        temperature: openai_req.temperature,
        top_p: openai_req.top_p,
        max_tokens: openai_req.max_tokens,
        tools: openai_req.tools.clone(),
    };

    Ok(dify_request)
}

fn transform_dify_to_openai_stream(dify_event: DifyEvent) -> OpenAIResponse {
    OpenAIResponse {
        id: dify_event.message_id,
        object: "chat.completion.chunk".to_string(),
        created: dify_event.created_at,
        model: "gpt-3.5-turbo".to_string(), // Default model
        choices: vec![OpenAIChoice {
            index: 0,
            delta: OpenAIDelta {
                role: None,
                content: Some(dify_event.answer),
                tool_calls: dify_event.tool_calls,
                files: dify_event.files,
            },
            finish_reason: None,
        }],
        usage: None,
    }
}

fn transform_dify_to_openai_blocking(dify_response: serde_json::Value) -> Result<OpenAIResponse, ChatCompletionError> {
    // Log the raw response for debugging
    debug!("Raw Dify response: {:?}", dify_response);

    let answer = dify_response["answer"]
        .as_str()
        .ok_or_else(|| ChatCompletionError::JsonSerializationError("Missing 'answer' field".to_string()))?
        .to_string();

    let message_id = dify_response["message_id"]
        .as_str()
        .ok_or_else(|| ChatCompletionError::JsonSerializationError("Missing 'message_id' field".to_string()))?
        .to_string();

    let created_at = dify_response["created_at"]
        .as_u64()
        .ok_or_else(|| ChatCompletionError::JsonSerializationError("Missing or invalid 'created_at' field".to_string()))?;

    Ok(OpenAIResponse {
        id: message_id,
        object: "chat.completion".to_string(),
        created: created_at,
        model: "gpt-3.5-turbo".to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            delta: OpenAIDelta {
                role: Some("assistant".to_string()),
                content: Some(answer),
                tool_calls: None, // We'll add tool calls support if needed
                files: None,
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: None,
    })
}

type StreamResponse = Pin<Box<dyn Stream<Item = Result<Bytes, actix_web::Error>> + Send>>;

fn process_chunk(chunk: Bytes) -> Result<Bytes, actix_web::Error> {
    let chunk_str = String::from_utf8_lossy(&chunk);
    let events: Vec<&str> = chunk_str.trim().split('\n').collect();
    
    let mut transformed_chunks = Vec::new();
    for event in events {
        if event.starts_with("data: ") {
            let data = &event[6..];
            if data == "[DONE]" {
                transformed_chunks.push(b"data: [DONE]\n\n".to_vec());
                continue;
            }

            if let Ok(dify_event) = serde_json::from_str::<DifyEvent>(data) {
                let openai_response = transform_dify_to_openai_stream(dify_event);
                if let Ok(json) = serde_json::to_string(&openai_response) {
                    transformed_chunks.push(format!("data: {}\n\n", json).into_bytes());
                }
            }
        }
    }

    Ok(Bytes::from(transformed_chunks.concat()))
}

fn handle_streaming_response(response: reqwest::Response) -> StreamResponse {
    Box::pin(response
        .bytes_stream()
        .map(|chunk_result| {
            chunk_result
                .map_err(|e| actix_web::error::ErrorInternalServerError(e))
                .and_then(process_chunk)
        }))
}

async fn send_request_to_dify(
    client: &Client,
    url: &str,
    api_key: &str,
    body: &DifyRequest,
    is_streaming: bool,
) -> Result<HttpResponse, ChatCompletionError> {
    let response = client.post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(body)
        .send()
        .await
        .map_err(|e| ChatCompletionError::DifyApiError(
            StatusCode::BAD_GATEWAY,
            e.to_string(),
        ))?;

    let status = response.status();
    
    if !status.is_success() {
        let error_text = response.text().await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(ChatCompletionError::DifyApiError(status, error_text));
    }

    if is_streaming {
        Ok(HttpResponse::Ok()
            .insert_header(("Content-Type", "text/event-stream"))
            .insert_header(("Cache-Control", "no-cache"))
            .insert_header(("Connection", "keep-alive"))
            .streaming(handle_streaming_response(response)))
    } else {
        // First get the raw JSON value to inspect it
        let raw_json = response.json::<serde_json::Value>().await
            .map_err(|e| ChatCompletionError::JsonSerializationError(format!("Failed to parse JSON: {}", e)))?;
        
        // Transform the raw JSON into OpenAI format
        let openai_response = transform_dify_to_openai_blocking(raw_json)?;
        Ok(HttpResponse::Ok().json(openai_response))
    }
}

pub async fn chat_completion(
    req: HttpRequest,
    body: web::Json<OpenAIRequest>,
) -> Result<HttpResponse, ChatCompletionError> {
    lazy_static! {
        static ref DIFY_URL: String = env::var("DIFY_URL").expect("DIFY_URL must be set");
    }

    let api_key = extract_api_key(&req)
        .ok_or(ChatCompletionError::InvalidApiKey)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| ChatCompletionError::RequestConstructionError(e.to_string()))?;

    let is_streaming = body.stream.unwrap_or(false);
    let dify_request = construct_dify_request(&body, body.user.clone()).await?;

    let final_url = format!("{}/v1/chat-messages", DIFY_URL.as_str());

    send_request_to_dify(&client, &final_url, &api_key, &dify_request, is_streaming).await
}
