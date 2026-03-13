use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Result, Context};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum Provider {
    OpenAICompat,
    Gemini,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    pub name: String,
    pub api_key: String,
    pub base_url: String,
    pub provider: Provider,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub models: HashMap<String, ModelConfig>,
    pub current_model: Option<String>,
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::get_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)?;
        serde_json::from_str(&content).context("Failed to parse config")
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // TODO: encrypt keys before saving to disk
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content).context("Failed to write config")
    }

    fn get_path() -> Result<PathBuf> {
        let mut path = dirs::config_dir().context("Could not find config directory")?;
        path.push("query.rs");
        path.push("config.json");
        Ok(path)
    }

    pub fn add_model(&mut self, provider: Provider, name: String, api_key: String, base_url: Option<String>) {
        let base_url = base_url.unwrap_or_else(|| match provider {
            Provider::Gemini => "https://generativelanguage.googleapis.com".to_string(),
            Provider::OpenAICompat => "https://api.openai.com/v1".to_string(),
        });

        self.models.insert(name.clone(), ModelConfig {
            name: name.clone(),
            api_key,
            base_url,
            provider,
        });
        if self.current_model.is_none() {
            self.current_model = Some(name);
        }
    }
}
