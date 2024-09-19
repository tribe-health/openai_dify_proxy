use chrono::Utc;
use serde_json::json;

use crate::features::dify::handlers::openai::types::{
    OpenAIRequest, OpenAIResponse, OpenAIChoice, OpenAIDelta,
    DifyRequest, DifyResponse, // Add these imports
};

pub fn construct_dify_request(openai_req: &OpenAIRequest) -> DifyRequest {
    let last_message = openai_req.messages.last().unwrap();
    let conversation_history = openai_req.messages[..openai_req.messages.len() - 1]
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<String>>()
        .join("\n");

    DifyRequest {
        inputs: json!({
            "conversation_history": conversation_history
        }),
        query: vec![last_message.content.to_string()],
        response_mode: if openai_req.stream { "streaming".to_string() } else { "blocking".to_string() },
        user: if openai_req.user.is_some() { openai_req.user.clone().unwrap() } else { "proxy".to_string() },
        temperature: openai_req.temperature,
        top_p: openai_req.top_p,
        max_tokens: openai_req.max_tokens,
        tools: Some(openai_req.tools.clone().unwrap_or_default()),
    }
}

pub fn transform_dify_to_openai(dify_response: &DifyResponse, original_request: &OpenAIRequest) -> OpenAIResponse {
    OpenAIResponse {
        id: format!("chatcmpl-{}", Utc::now().timestamp_millis()),
        object: "chat.completion.chunk".to_string(),
        created: Utc::now().timestamp() as u64,
        model: original_request.model.clone().unwrap_or_else(|| "dify-transformed".to_string()),
        choices: vec![OpenAIChoice {
            index: 0,
            delta: OpenAIDelta {
                content: Some(dify_response.answer.clone()),
                tool_calls: dify_response.tool_calls.clone(),
                files: dify_response.files.clone(),
            },
            finish_reason: None,
        }],
        usage: None,
    }
}

pub fn create_final_chunk() -> OpenAIResponse {
    OpenAIResponse {
        id: format!("chatcmpl-{}", Utc::now().timestamp_millis()),
        object: "chat.completion.chunk".to_string(),
        created: Utc::now().timestamp() as u64,
        model: "dify-transformed".to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            delta: OpenAIDelta {
                content: None,
                tool_calls: None,
                files: None,
            },
            finish_reason: Some("stop".to_string()),
        }],
        usage: None,
    }
}

pub fn create_error_response(message: &str) -> OpenAIResponse {
    OpenAIResponse {
        id: format!("chatcmpl-error-{}", Utc::now().timestamp_millis()),
        object: "chat.completion.chunk".to_string(),
        created: Utc::now().timestamp() as u64,
        model: "dify-transformed".to_string(),
        choices: vec![OpenAIChoice {
            index: 0,
            delta: OpenAIDelta {
                content: Some(format!("Error: {}", message)),
                tool_calls: None,
                files: None,
            },
            finish_reason: Some("error".to_string()),
        }],
        usage: None,
    }
}