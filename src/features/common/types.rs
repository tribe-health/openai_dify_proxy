use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone )]
pub struct OpenAIMessage {
    pub role: String,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum MessageContent {
    String(String),
    Complex(Vec<ComplexMessageContent>),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ComplexMessageContent {
    pub r#type: String,
    pub text: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAIRequest {
    pub messages: Vec<OpenAIMessage>,
    #[serde(default)]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Tool {
    pub r#type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DifyRequest {
    pub inputs: serde_json::Value,
    pub query: Vec<String>,
    pub response_mode: String,
    pub user: String,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<Tool>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DifyResponse {
    pub event: String,
    pub task_id: String,
    pub conversation_id: String,
    pub message_id: String,
    pub created_at: u64,
    pub answer: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub files: Option<Vec<File>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub r#type: String,
    pub url: String,
    pub name: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct OpenAIResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChoice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Serialize, Clone)]
pub struct OpenAIChoice {
    pub index: u32,
    pub delta: OpenAIDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct OpenAIDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<File>>,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct DifyEvent {
    pub event: String,
    pub task_id: String,
    pub conversation_id: String,
    pub message_id: String,
    pub created_at: u64,
    pub answer: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub files: Option<Vec<File>>
}