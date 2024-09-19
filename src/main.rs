use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::env;
use log::{info, error};
use chrono;

#[derive(Debug, Deserialize, Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIRequest {
    messages: Vec<OpenAIMessage>,
    stream: Option<bool>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    max_tokens: Option<u32>,
    tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize)]
struct DifyRequest {
    inputs: serde_json::Value,
    query: Vec<String>,
    response_mode: String,
    user: Option<String>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    max_tokens: Option<u32>,
    tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize)]
struct OpenAIResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: Usage,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpenAIChoice {
    index: u32,
    message: OpenAIMessage,
    finish_reason: String,
}

#[derive(Debug, Serialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

async fn chat_completion(req: web::Json<OpenAIRequest>) -> impl Responder {
    info!("Received POST request to /v1/chat/completions");
    info!("Input from OpenAI client: {:?}", req);

    let dify_request = construct_dify_request(&req);
    info!("Request to Dify: {:?}", dify_request);

    let client = Client::new();
    let dify_api_url = env::var("DIFY_API_URL").expect("DIFY_API_URL must be set");
    let dify_api_key = env::var("DIFY_API_KEY").expect("DIFY_API_KEY must be set");

    let response = client.post(format!("{}/chat-messages", dify_api_url))
        .header("Authorization", format!("Bearer {}", dify_api_key))
        .json(&dify_request)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let dify_response = resp.json::<serde_json::Value>().await.unwrap();
                info!("Response from Dify: {:?}", dify_response);

                let openai_response = transform_dify_to_openai(&dify_response);
                info!("Transformed response to OpenAI format: {:?}", openai_response);

                HttpResponse::Ok().json(openai_response)
            } else {
                let error_message = format!("Dify API responded with status {}", resp.status());
                error!("{}", error_message);
                HttpResponse::InternalServerError().body(error_message)
            }
        }
        Err(e) => {
            let error_message = format!("Error calling Dify API: {}", e);
            error!("{}", error_message);
            HttpResponse::InternalServerError().body(error_message)
        }
    }
}

fn construct_dify_request(openai_req: &OpenAIRequest) -> DifyRequest {
    let last_message = openai_req.messages.last().unwrap();
    let conversation_history = openai_req.messages[..openai_req.messages.len() - 1]
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<String>>()
        .join("\n");

    DifyRequest {
        inputs: serde_json::json!({
            "conversation_history": conversation_history
        }),
        query: vec![last_message.content.to_string()],
        response_mode: if openai_req.stream.unwrap_or(false) { String::from("streaming") } else { String::from("blocking") },
        user: None,
        temperature: openai_req.temperature,
        top_p: openai_req.top_p,
        max_tokens: openai_req.max_tokens,
        tools: openai_req.tools.clone(),
    }
}

fn transform_dify_to_openai(dify_response: &serde_json::Value) -> OpenAIResponse {
    OpenAIResponse {
        id: format!("chatcmpl-{}", chrono::Utc::now().timestamp()),
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp() as u64,
        model: "dify-transformed".to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            message: OpenAIMessage {
                role: "assistant".to_string(),
                content: dify_response["answer"].as_str().unwrap_or("").to_string(),
            },
            finish_reason: "stop".to_string(),
        }],
        usage: Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        },
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            .route("/v1/chat/completions", web::post().to(chat_completion))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}