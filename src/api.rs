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
    usage: Option<Usage>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
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
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: u32,
    #[serde(rename = "totalTokenCount")]
    total_token_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Debug, Clone)]
pub enum ApiResult {
    Text(String, Usage),
    ToolCall(Message, String, Value, Usage), // Assistant message to save, tool name, arguments, usage
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
            Provider::Anthropic => self_anthropic(&self.client, config, messages, tools).await,
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
    let usage = body.usage.clone().unwrap_or_default();
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
            return Ok(ApiResult::ToolCall(assistant_msg, tc.function.name.clone(), args, usage));
        }
    }
    if let Some(content) = choice.message.content.as_ref() {
        return Ok(ApiResult::Text(content.clone(), usage));
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
    let usage = body.usage_metadata.as_ref().map(|u| Usage {
        prompt_tokens: u.prompt_token_count,
        completion_tokens: u.candidates_token_count,
        total_tokens: u.total_token_count,
    }).unwrap_or_default();

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
            return Ok(ApiResult::ToolCall(assistant_msg, fc.name.clone(), fc.args.clone(), usage));
        }
    }
    
    if let Some(part) = candidate.content.parts.iter().find(|p| p.text.is_some()) {
        if let Some(text) = &part.text {
            return Ok(ApiResult::Text(text.clone(), usage));
        }
    }
    
    Err(anyhow::anyhow!("No text or function call in Gemini response"))
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContentPart>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum AnthropicContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: Value },
    #[serde(rename = "tool_result")]
    ToolResult { tool_use_id: String, content: String },
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicTool {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    input_schema: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentPart>,
    usage: AnthropicUsage,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

async fn self_anthropic(client: &Client, config: &ModelConfig, messages: Vec<Message>, tools: Vec<Tool>) -> Result<ApiResult> {
    let url = "https://api.anthropic.com/v1/messages";
    
    let anthropic_tools = if tools.is_empty() {
        None
    } else {
        Some(tools.into_iter().map(|t| AnthropicTool {
            name: t.name.to_string(),
            description: t.description.map(|d| d.to_string()),
            input_schema: Value::Object(t.input_schema.as_ref().clone()),
        }).collect())
    };

    let anthropic_messages = messages.into_iter().map(|m| {
        let mut parts = Vec::new();
        if let Some(text) = m.content.as_ref() {
            if m.tool_call_id.is_some() {
                parts.push(AnthropicContentPart::ToolResult {
                    tool_use_id: m.tool_call_id.clone().unwrap(),
                    content: text.clone(),
                });
            } else {
                parts.push(AnthropicContentPart::Text { text: text.clone() });
            }
        }
        if let Some(tool_calls) = m.tool_calls.as_ref() {
            for tc in tool_calls {
                parts.push(AnthropicContentPart::ToolUse {
                    id: tc.id.clone(),
                    name: tc.function.name.clone(),
                    input: serde_json::from_str(&tc.function.arguments).unwrap_or(Value::Null),
                });
            }
        }
        AnthropicMessage {
            role: if m.role == "assistant" { "assistant".to_string() } else { "user".to_string() },
            content: parts,
        }
    }).collect();

    let request = AnthropicRequest {
        model: config.name.clone(),
        messages: anthropic_messages,
        max_tokens: 4096,
        tools: anthropic_tools,
    };

    let response = client
        .post(url)
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error = response.text().await?;
        return Err(anyhow::anyhow!("Anthropic API Error: {}", error));
    }

    let body = response.json::<AnthropicResponse>().await?;
    let usage = Usage {
        prompt_tokens: body.usage.input_tokens,
        completion_tokens: body.usage.output_tokens,
        total_tokens: body.usage.input_tokens + body.usage.output_tokens,
    };

    if let Some(part) = body.content.iter().find(|p| matches!(p, AnthropicContentPart::ToolUse { .. })) {
        if let AnthropicContentPart::ToolUse { id, name, input } = part {
            let tc = ToolCall {
                id: id.clone(),
                r#type: "function".to_string(),
                function: ToolCallFunction {
                    name: name.clone(),
                    arguments: input.to_string(),
                },
            };
            let assistant_msg = Message {
                role: "assistant".to_string(),
                content: None,
                tool_calls: Some(vec![tc]),
                tool_call_id: None,
                name: None,
            };
            return Ok(ApiResult::ToolCall(assistant_msg, name.clone(), input.clone(), usage));
        }
    }

    if let Some(part) = body.content.iter().find(|p| matches!(p, AnthropicContentPart::Text { .. })) {
        if let AnthropicContentPart::Text { text } = part {
            return Ok(ApiResult::Text(text.clone(), usage));
        }
    }

    Err(anyhow::anyhow!("No text or tool use in Anthropic response"))
}
