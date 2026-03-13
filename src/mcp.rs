use rmcp::service::{RoleClient, serve_client, RunningService, Peer};
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::model::{Tool, CallToolResult, CallToolRequestParams};
use crate::config::McpServerConfig;
use anyhow::{Result, Context};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::process::Command;
use std::borrow::Cow;

pub type McpClientHandle = RunningService<RoleClient, ()>;

pub struct McpManager {
    /// server_name -> client_handle
    pub clients: Arc<Mutex<HashMap<String, McpClientHandle>>>,
    /// namespaced_tool_name (server:tool) -> server_name
    pub tool_cache: Arc<Mutex<HashMap<String, String>>>,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            tool_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn add_server(&self, name: &str, config: &McpServerConfig) -> Result<()> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);
        for (k, v) in &config.env {
            cmd.env(k, v);
        }
        
        let transport = TokioChildProcess::new(cmd).context("Failed to spawn MCP server")?;
        let running_service: McpClientHandle = serve_client((), transport).await
            .map_err(|e| anyhow::anyhow!("MCP Init Error: {:?}", e))?;
        
        self.clients.lock().await.insert(name.to_string(), running_service);
        // Refresh cache after adding
        let _ = self.list_tools().await;
        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        let mut all_tools = Vec::new();
        let mut new_cache = HashMap::new();
        
        // Get peers while holding lock for a short time
        let peers: Vec<(String, Peer<RoleClient>)> = {
            let clients = self.clients.lock().await;
            clients.iter().map(|(n, c)| (n.clone(), c.peer().clone())).collect()
        };
        
        for (server_name, peer) in peers {
            let tools = peer.list_all_tools().await
                .map_err(|e| anyhow::anyhow!("MCP ListTools Error from {}: {:?}", server_name, e))?;
            
            for mut tool in tools {
                let original_name = tool.name.to_string();
                let namespaced_name = format!("{}:{}", server_name, original_name);
                
                tool.name = Cow::Owned(namespaced_name.clone());
                new_cache.insert(namespaced_name, server_name.clone());
                all_tools.push(tool);
            }
        }
        
        *self.tool_cache.lock().await = new_cache;
        Ok(all_tools)
    }

    pub async fn call_tool(&self, namespaced_name: &str, arguments: serde_json::Value) -> Result<CallToolResult> {
        let server_name = self.tool_cache.lock().await.get(namespaced_name).cloned()
            .context(format!("Tool {} not found in cache", namespaced_name))?;
            
        let peer = {
            let clients = self.clients.lock().await;
            clients.get(&server_name).map(|c| c.peer().clone())
                .context(format!("Server {} not found for tool {}", server_name, namespaced_name))?
        };
        
        let actual_name = namespaced_name.splitn(2, ':').nth(1).unwrap_or(namespaced_name);
        
        let mut params = CallToolRequestParams::new(Cow::Owned(actual_name.to_string()));
        if let serde_json::Value::Object(map) = arguments {
            params.arguments = Some(map);
        }
        
        peer.call_tool(params).await.map_err(|e| anyhow::anyhow!("MCP CallTool Error: {:?}", e))
    }

    pub async fn shutdown(&self) {
        let mut clients = self.clients.lock().await;
        for (_name, client) in clients.drain() {
            drop(client);
        }
        self.tool_cache.lock().await.clear();
    }
}

impl Clone for McpManager {
    fn clone(&self) -> Self {
        Self {
            clients: Arc::clone(&self.clients),
            tool_cache: Arc::clone(&self.tool_cache),
        }
    }
}
