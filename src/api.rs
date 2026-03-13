use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use serde_json::Value;
use reqwest::Client;
use crate::config::{ModelConfig, Provider};
use rmcp::model::Tool;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    pub fn new(role: &str, content: &str) -> Self {
        Self {
            role: role.to_string(),
            content: Some(content.to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub fn new_tool_response(name: &str, tool_call_id: &str, content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content.to_string()),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.to_string()),
            name: Some(name.to_string()),
        }
    }

    pub fn content_text(&self) -> String {
        self.content.as_deref().unwrap_or("").to_string()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiTool>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiTool {
    r#type: String,
    function: OpenAiFunction,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiFunction {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    parameters: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Choice {
    message: MessageWithToolCalls,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageWithToolCalls {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GeminiTool>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiTool {
    function_declarations: Vec<GeminiFunctionDeclaration>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiFunctionDeclaration {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeminiPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<GeminiFunctionCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_response: Option<GeminiFunctionResponse>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeminiFunctionCall {
    pub name: String,
    pub args: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeminiFunctionResponse {
    pub name: String,
    pub response: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Debug, Clone)]
pub enum ApiResult {
    Text(String),
    ToolCall(Message, String, Value), // Assistant message to save, tool name, arguments
}

pub struct ApiClient {
    client: Client,
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn send_chat_completion(
        &self,
        config: &ModelConfig,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<ApiResult> {
        match config.provider {
            Provider::OpenAICompat => self_openai(&self.client, config, messages, tools).await,
            Provider::Gemini => self_gemini(&self.client, config, messages, tools).await,
        }
    }
}

async fn self_openai(client: &Client, config: &ModelConfig, messages: Vec<Message>, tools: Vec<Tool>) -> Result<ApiResult> {
    let url = format!("{}/chat/completions", config.base_url);
    
    let openai_tools = if tools.is_empty() {
        None
    } else {
        Some(tools.into_iter().map(|t| OpenAiTool {
            r#type: "function".to_string(),
            function: OpenAiFunction {
                name: t.name.to_string(),
                description: t.description.map(|d| d.to_string()),
                parameters: Value::Object(t.input_schema.as_ref().clone()),
            }
        }).collect())
    };

    let request = ChatCompletionRequest {
        model: config.name.clone(),
        messages,
        tools: openai_tools,
    };

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error = response.text().await?;
        return Err(anyhow::anyhow!("API Error: {}", error));
    }

    let body = response.json::<ChatCompletionResponse>().await?;
    let choice = body.choices.get(0).context("No choice in response")?;
    
    if let Some(tool_calls) = choice.message.tool_calls.as_ref() {
        if let Some(tc) = tool_calls.first() {
            let args = serde_json::from_str(&tc.function.arguments)?;
            let assistant_msg = Message {
                role: "assistant".to_string(),
                content: choice.message.content.clone(),
                tool_calls: Some(tool_calls.clone()),
                tool_call_id: None,
                name: None,
            };
            return Ok(ApiResult::ToolCall(assistant_msg, tc.function.name.clone(), args));
        }
    }
    if let Some(content) = choice.message.content.as_ref() {
        return Ok(ApiResult::Text(content.clone()));
    }
    
    Err(anyhow::anyhow!("No content or tool calls in OpenAI response"))
}

async fn self_gemini(client: &Client, config: &ModelConfig, messages: Vec<Message>, tools: Vec<Tool>) -> Result<ApiResult> {
    let url = format!("{}/v1beta/models/{}:generateContent?key={}", config.base_url, config.name, config.api_key);
    
    let gemini_tools = if tools.is_empty() {
        None
    } else {
        let function_declarations = tools.into_iter().map(|t| GeminiFunctionDeclaration {
            name: t.name.to_string(),
            description: t.description.map(|d| d.to_string()).unwrap_or_default(),
            parameters: Value::Object(t.input_schema.as_ref().clone()),
        }).collect();
        Some(vec![GeminiTool { function_declarations }])
    };

    let contents = messages.into_iter().map(|m| {
        let mut parts = Vec::new();
        if let Some(text) = m.content.as_ref() {
            parts.push(GeminiPart { text: Some(text.clone()), function_call: None, function_response: None });
        }
        if let Some(tool_calls) = m.tool_calls.as_ref() {
            for tc in tool_calls {
                parts.push(GeminiPart {
                    text: None,
                    function_call: Some(GeminiFunctionCall {
                        name: tc.function.name.clone(),
                        args: serde_json::from_str(&tc.function.arguments).unwrap_or(Value::Null),
                    }),
                    function_response: None,
                });
            }
        }
        if m.tool_call_id.is_some() {
            // For Gemini, tool response corresponds to function_response
            parts.push(GeminiPart {
                text: None,
                function_call: None,
                function_response: Some(GeminiFunctionResponse {
                    name: m.name.clone().unwrap_or_default(),
                    response: serde_json::json!({ "result": m.content_text() }),
                }),
            });
        }

        GeminiContent {
            role: if m.role == "assistant" { "model".to_string() } else { "user".to_string() },
            parts,
        }
    }).collect();

    let request = GeminiRequest { contents, tools: gemini_tools };

    let response = client
        .post(url)
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_json: Value = response.json().await?;
        let error_msg = error_json["error"]["message"].as_str().unwrap_or("Unknown error");
        return Err(anyhow::anyhow!("Gemini API Error: {}", error_msg));
    }

    let body = response.json::<GeminiResponse>().await?;
    let candidate = body.candidates.get(0).context("No candidate in Gemini response")?;
    if let Some(part) = candidate.content.parts.iter().find(|p| p.function_call.is_some()) {
        if let Some(fc) = &part.function_call {
            let tc = ToolCall {
                id: "gemini-call".to_string(), // Gemini doesn't always provide IDs in the same way
                r#type: "function".to_string(),
                function: ToolCallFunction {
                    name: fc.name.clone(),
                    arguments: fc.args.to_string(),
                },
            };
            let assistant_msg = Message {
                role: "assistant".to_string(),
                content: None,
                tool_calls: Some(vec![tc]),
                tool_call_id: None,
                name: None,
            };
            return Ok(ApiResult::ToolCall(assistant_msg, fc.name.clone(), fc.args.clone()));
        }
    }
    
    if let Some(part) = candidate.content.parts.iter().find(|p| p.text.is_some()) {
        if let Some(text) = &part.text {
            return Ok(ApiResult::Text(text.clone()));
        }
    }
    
    Err(anyhow::anyhow!("No text or function call in Gemini response"))
}
