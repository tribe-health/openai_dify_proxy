use chrono::Utc;
use serde_json::json;
use log::{info, warn};

use crate::features::dify::handlers::openai::types::{
    OpenAIRequest, OpenAIResponse, OpenAIChoice, OpenAIDelta,
    DifyRequest, DifyResponse, MessageContent,
};

pub fn construct_dify_request(openai_req: &OpenAIRequest) -> Result<DifyRequest, String> {
    info!("Constructing Dify request from OpenAI request");

    if openai_req.messages.is_empty() {
        return Err("OpenAI request contains no messages".to_string());
    }

    let last_message = openai_req.messages.last().unwrap();
    let conversation_history = openai_req.messages[..openai_req.messages.len() - 1]
        .iter()
        .map(|m| format!("{}: {}", m.role, message_content_to_string(&m.content)))
        .collect::<Vec<String>>()
        .join("\n");

    let query = vec![message_content_to_string(&last_message.content)];

    if query[0].is_empty() {
        warn!("Last message in OpenAI request contains no content");
    }

    let dify_request = DifyRequest {
        inputs: json!({
            "conversation_history": conversation_history
        }),
        query,
        response_mode: if openai_req.stream.unwrap_or(true) { "streaming".to_string() } else { "blocking".to_string() },
        user: openai_req.user.clone().unwrap_or_else(|| "proxy".to_string()),
        temperature: openai_req.temperature,
        top_p: openai_req.top_p,
        max_tokens: openai_req.max_tokens,
        tools: openai_req.tools.clone(),
    };

    info!("Dify request constructed successfully: {:?}", dify_request);
    Ok(dify_request)
}

fn message_content_to_string(content: &MessageContent) -> String {
    match content {
        MessageContent::String(s) => s.clone(),
        MessageContent::Complex(complex) => complex.iter().map(|c| c.text.clone()).collect::<Vec<_>>().join(" "),
    }
}

pub fn transform_dify_to_openai(dify_response: &DifyResponse, original_request: &OpenAIRequest) -> OpenAIResponse {
    info!("Transforming Dify response to OpenAI response");
    OpenAIResponse {
        id: format!("chatcmpl-{}", Utc::now().timestamp_millis()),
        object: "chat.completion".to_string(),
        created: Utc::now().timestamp() as u64,
        model: original_request.model.clone().unwrap_or_else(|| "dify-transformed".to_string()),
        choices: vec![OpenAIChoice {
            index: 0,
            delta: OpenAIDelta {
                role: Some("assistant".to_string()),
                content: Some(dify_response.answer.clone()),
                tool_calls: dify_response.tool_calls.clone(),
                files: dify_response.files.clone(),
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: None,
    }
}

pub fn transform_dify_to_openai_chunk(dify_response: &str, original_request: &OpenAIRequest) -> OpenAIResponse {
    info!("Transforming Dify chunk to OpenAI chunk");
    
    // Check if the response starts with "Error:"
    if dify_response.trim().starts_with("Error:") {
        return create_error_response(dify_response);
    }

    // Rest of the function remains the same
    OpenAIResponse {
        id: format!("chatcmpl-{}", Utc::now().timestamp_millis()),
        object: "chat.completion.chunk".to_string(),
        created: Utc::now().timestamp() as u64,
        model: original_request.model.clone().unwrap_or_else(|| "dify-transformed".to_string()),
        choices: vec![OpenAIChoice {
            index: 0,
            delta: OpenAIDelta {
                role: Some("assistant".to_string()),
                content: Some(dify_response.to_string()),
                tool_calls: None,
                files: None,
            },
            finish_reason: None,
        }],
        usage: None,
    }
}

pub fn create_final_chunk() -> OpenAIResponse {
    info!("Creating final OpenAI chunk");
    OpenAIResponse {
        id: format!("chatcmpl-{}", Utc::now().timestamp_millis()),
        object: "chat.completion.chunk".to_string(),
        created: Utc::now().timestamp() as u64,
        model: "dify-transformed".to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            delta: OpenAIDelta {
                role: None,
                content: None,
                tool_calls: None,
                files: None,
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: None,
    }
}

// Modify create_error_response to handle both string and borrowed str
pub fn create_error_response(message: &str) -> OpenAIResponse {
    warn!("Creating error response: {}", message);
    OpenAIResponse {
        id: format!("chatcmpl-error-{}", Utc::now().timestamp_millis()),
        object: "chat.completion.chunk".to_string(),
        created: Utc::now().timestamp() as u64,
        model: "dify-transformed".to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            delta: OpenAIDelta {
                role: Some("assistant".to_string()),
                content: Some(message.to_string()), // Use the entire message
                tool_calls: None,
                files: None,
            },
            finish_reason: Some("error".to_string()),
        }],
        usage: None,
    }
}
