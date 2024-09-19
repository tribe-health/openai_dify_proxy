use actix_web::{web, HttpResponse, Responder};
use reqwest::Client;
use log::{info, error};
use actix_web::web::Bytes;
use futures_util::StreamExt;
use serde_json::Value;
use crate::features::app::app_state::AppState;
use crate::features::dify::handlers::openai::types::OpenAIRequest;
use crate::features::dify::handlers::openai::transform::{
    construct_dify_request, transform_dify_to_openai_chunk, create_final_chunk, create_error_response,
    transform_dify_to_openai
};
use crate::utils::status::reqwest_to_actix_status;
use crate::features::dify::handlers::openai::types::DifyResponse;

pub async fn chat_completion(
    req: web::Json<OpenAIRequest>,
    data: web::Data<AppState>
) -> impl Responder {
    info!("Received POST request to /v1/chat/completions");
    info!("Input from OpenAI client: {:?}", req);

    let dify_request = construct_dify_request(&req);
    info!("Request to Dify: {:?}", dify_request);

    let client = Client::new();

    let response = client.post(format!("{}/chat-messages", data.dify_api_url))
        .header("Authorization", format!("Bearer {}", data.dify_api_key))
        .json(&dify_request)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_else(|_| "No response body".to_string());
                error!("Dify API responded with status {}: {}", status, body);
                return HttpResponse::build(reqwest_to_actix_status(status))
                    .content_type("application/json")
                    .body(body);
            }

            info!("Dify API responded with status {}", status);
            if req.stream.unwrap_or(true) {
                handle_streaming_response(resp, req.into_inner()).await
            } else {
                handle_blocking_response(resp, req.into_inner()).await
            }
        }
        Err(e) => {
            let error_message = format!("Error calling Dify API: {}", e);
            error!("{}", error_message);
            HttpResponse::InternalServerError()
                .content_type("application/json")
                .body(error_message)
        }
    }
}

async fn handle_streaming_response(resp: reqwest::Response, original_request: OpenAIRequest) -> HttpResponse {
    info!("Streaming response from Dify API");
    let stream = resp.bytes_stream().flat_map(move |chunk| {
        futures_util::stream::iter(chunk.map(|bytes| {
            let chunk_str = String::from_utf8_lossy(&bytes);
            let lines = chunk_str.split('\n').filter(|line| !line.trim().is_empty());
            
            lines.flat_map(|line| {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        Some(Bytes::from("data: [DONE]\n\n"))
                    } else {
                        match serde_json::from_str::<Value>(data) {
                            Ok(parsed) => {
                                // Log all messages
                                info!("Received Dify message: {}", serde_json::to_string(&parsed).unwrap());
                                
                                // Only transform and forward "message" events
                                if parsed["event"] == "message" {
                                    if let Some(answer) = parsed["answer"].as_str() {
                                        let transformed = transform_dify_to_openai_chunk(answer, &original_request);
                                        Some(Bytes::from(format!("data: {}\n\n", serde_json::to_string(&transformed).unwrap())))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            }
                            Err(e) => {
                                error!("Error parsing JSON: {}", e);
                                let error_response = create_error_response(&format!("Error parsing JSON: {}", e));
                                Some(Bytes::from(format!("data: {}\n\n", serde_json::to_string(&error_response).unwrap())))
                            }
                        }
                    }
                } else {
                    None
                }
            }).collect::<Vec<Bytes>>()
        }).unwrap_or_else(|e| {
            error!("Error processing stream: {}", e);
            let error_response = create_error_response(&format!("Error processing stream: {}", e));
            vec![Bytes::from(format!("data: {}\n\n", serde_json::to_string(&error_response).unwrap()))]
        }))
    }).map(Ok::<_, std::convert::Infallible>);

    // Append the final chunk and [DONE] message after the stream ends
    let stream_with_final_chunk = stream.chain(futures_util::stream::iter(vec![
        Ok(Bytes::from(format!("data: {}\n\n", serde_json::to_string(&create_final_chunk()).unwrap()))),
        Ok(Bytes::from("data: [DONE]\n\n"))
    ]));

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(stream_with_final_chunk)
}

async fn handle_blocking_response(resp: reqwest::Response, original_request: OpenAIRequest) -> HttpResponse {
    info!("Blocking response from Dify API");
    match resp.json::<DifyResponse>().await {
        Ok(dify_response) => {
            let openai_response = transform_dify_to_openai(&dify_response, &original_request);
            HttpResponse::Ok()
                .content_type("application/json")
                .json(openai_response)
        }
        Err(e) => {
            let error_message = format!("Error parsing Dify response: {}", e);
            error!("{}", error_message);
            let error_response = create_error_response(&error_message);
            HttpResponse::InternalServerError()
                .content_type("application/json")
                .json(error_response)
        }
    }
}