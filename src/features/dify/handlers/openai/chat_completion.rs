use actix_web::error::ErrorInternalServerError;
use actix_web::{web, HttpResponse, HttpRequest};
use awc::Client;
use serde_json::Value;
use actix_web::http::StatusCode as ActixStatusCode;
use log::{info, error};
use futures_util::{StreamExt, stream::once};
use crate::features::app::app_state::AppState;
use crate::features::common::types::{OpenAIRequest, DifyResponse, DifyEvent};
use crate::features::dify::handlers::openai::transform::{
    construct_dify_request, create_error_response, transform_dify_to_openai, transform_dify_to_openai_chunk
};
use bytes::Bytes;
use serde_json;
use futures::future::ready;
use crate::features::common::types::DifyRequest;
use crate::features::common::types::ChatCompletionPayload;

fn reqwest_to_actix_status(status: ReqwestStatusCode) -> ActixStatusCode {
    ActixStatusCode::from_u16(status.as_u16()).unwrap_or(ActixStatusCode::INTERNAL_SERVER_ERROR)
}

fn extract_api_key(http_req: &HttpRequest) -> Option<&str> {
    http_req.headers().get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer "))
}

async fn construct_dify_request_or_error(req: &web::Json<OpenAIRequest>) -> Result<DifyRequest, HttpResponse> {
    construct_dify_request(req).map_err(|e| {
        error!("Failed to construct Dify request: {}", e);
        HttpResponse::BadRequest()
            .content_type("application/json")
            .json(create_error_response(&format!("Failed to construct Dify request: {}", e)))
    })
}

async fn handle_dify_response(resp: reqwest::Response, original_request: OpenAIRequest) -> Result<HttpResponse, actix_web::Error> {
    if original_request.stream.unwrap_or(true) {
        handle_streaming_response(resp, original_request).await
    } else {
        handle_blocking_response(resp, original_request).await
    }
}

pub async fn chat_completion(
    payload: web::Json<ChatCompletionPayload>,
    config: web::Data<Config>,
) -> Result<HttpResponse, Error> {
    let client = Client::default();
    let url = config.dify_api_url.clone();

    let mut response = client.post(url)
        .insert_header(("Authorization", format!("Bearer {}", config.openai_api_key)))
        .insert_header(("Content-Type", "application/json"))
        .send_json(&payload)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let body: Value = response.json().await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    Ok(HttpResponse::Ok().json(body))
}

async fn handle_streaming_response(resp: awc::ClientResponse, original_request: OpenAIRequest) -> Result<HttpResponse, actix_web::Error> {
    info!("Streaming response from Dify API");

    let original_request = original_request.clone();
    let transformed_stream = resp.bytes_stream().filter_map(move |chunk| {
        ready(match chunk {
            Ok(data) => {
                if let Ok(chunk_str) = String::from_utf8(data.to_vec()) {
                    info!("Received chunk from Dify: {}", chunk_str);

                    let json_str = chunk_str.trim_start_matches("data: ").trim();

                    match serde_json::from_str::<DifyEvent>(json_str) {
                        Ok(dify_event) => {
                            if dify_event.event == "message" {
                                let answer = &dify_event.answer;

                                if !answer.trim().is_empty() {
                                    let transformed = transform_dify_to_openai_chunk(answer, &original_request);
                                    match serde_json::to_string(&transformed) {
                                        Ok(json_string) => Some(Ok::<Bytes, actix_web::Error>(Bytes::from(format!("data: {}\n\n", json_string)))),
                                        Err(e) => Some(Err(actix_web::error::ErrorInternalServerError(format!("JSON serialization error: {}", e))))
                                    }
                                } else {
                                    None
                                }
                            } else {
                                info!("Received non-message event from Dify: {:?}", dify_event);
                                None
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse Dify chunk as JSON: {}", e);
                            None
                        }
                    }
                } else {
                    error!("Failed to convert chunk to UTF-8 string");
                    None
                }
            }
            Err(e) => {
                error!("Error while streaming data: {}", e);
                None
            }
        })
    });

    let final_stream = transformed_stream.chain(once(async {
        Ok(Bytes::from("data: [DONE]\n\n"))
    }));

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(final_stream))
}

async fn handle_blocking_response(resp: awc::ConnectResponse, original_request: OpenAIRequest) -> Result<HttpResponse, actix_web::Error> {
    info!("Blocking response from Dify API");
    let body = resp.into_body().await?.to_vec();
    match serde_json::from_slice::<DifyResponse>(&body) {
        Ok(dify_response) => {
            let openai_response = transform_dify_to_openai(&dify_response, &original_request);
            Ok(HttpResponse::Ok()
                .content_type("application/json")
                .json(openai_response))
        }
        Err(e) => {
            let error_message = format!("Error parsing Dify response: {}", e);
            error!("{}", error_message);
            let error_response = create_error_response(&error_message);
            Ok(HttpResponse::InternalServerError()
                .content_type("application/json")
                .json(error_response))
        }
    }
}
