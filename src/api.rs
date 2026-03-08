use serde::{Deserialize, Serialize};
use anyhow::Result;
use reqwest::Client;
use crate::config::{ModelConfig, Provider};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
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

    pub fn send_chat_completion(
        &self,
        config: &ModelConfig,
        messages: Vec<Message>,
    ) -> impl std::future::Future<Output = Result<String>> + Send {
        let client = self.client.clone();
        let config = config.clone();
        let messages = messages.clone();

        async move {
            match config.provider {
                Provider::OpenAICompat => self_openai(&client, &config, messages).await,
                Provider::Gemini => self_gemini(&client, &config, messages).await,
            }
        }
    }
}

async fn self_openai(client: &Client, config: &ModelConfig, messages: Vec<Message>) -> Result<String> {
    let url = format!("{}/chat/completions", config.base_url);
    let request = ChatCompletionRequest {
        model: config.name.clone(),
        messages,
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
    Ok(body.choices.get(0).map(|c| c.message.content.clone()).unwrap_or_default())
}

async fn self_gemini(client: &Client, config: &ModelConfig, messages: Vec<Message>) -> Result<String> {
    let url = format!("{}/v1beta/models/{}:generateContent?key={}", config.base_url, config.name, config.api_key);
    
    let contents = messages.into_iter().map(|m| {
        GeminiContent {
            role: if m.role == "assistant" { "model".to_string() } else { "user".to_string() },
            parts: vec![GeminiPart { text: m.content }],
        }
    }).collect();

    let request = GeminiRequest { contents };

    let response = client
        .post(url)
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_json: serde_json::Value = response.json().await?;
        let error_msg = error_json["error"]["message"].as_str().unwrap_or("Unknown error");
        return Err(anyhow::anyhow!("Gemini API Error: {}", error_msg));
    }

    let body = response.json::<GeminiResponse>().await?;
    Ok(body.candidates.get(0).map(|c| c.content.parts.get(0).map(|p| p.text.clone()).unwrap_or_default()).unwrap_or_default())
}
