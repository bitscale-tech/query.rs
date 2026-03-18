use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use std::fs;
use std::path::PathBuf;
use crate::api::Message;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Memory {
    pub interactions: Vec<Interaction>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Interaction {
    pub timestamp: String,
    pub messages: Vec<Message>,
}

impl Memory {
    pub fn load() -> Result<Self> {
        let path = Self::get_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)?;
        serde_json::from_str(&content).context("Failed to parse memory")
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content).context("Failed to write memory")
    }

    fn get_path() -> Result<PathBuf> {
        let mut path = dirs::config_dir().context("Could not find config directory")?;
        path.push("query.rs");
        path.push("memory.json");
        Ok(path)
    }

    pub fn add_interaction(&mut self, messages: Vec<Message>) {
        let timestamp = chrono::Local::now().to_rfc3339();
        self.interactions.push(Interaction { timestamp, messages });
        
        // Limit memory size?
        if self.interactions.len() > 50 {
            self.interactions.remove(0);
        }
    }
}
