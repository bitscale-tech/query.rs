mod config;
mod api;
mod mcp;
mod ui;
mod memory;

use memory::Memory;

use anyhow::Result;
use api::{ApiClient, Message};
use config::Config;
use mcp::McpManager;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    Terminal,
};
use dotenvy;
use std::io;
use std::time::Duration;
use std::sync::Arc;

pub(crate) struct App {
    pub(crate) config: Config,
    pub(crate) messages: Vec<Message>,
    pub(crate) input: String,
    pub(crate) status_message: String,
    pub(crate) is_loading: bool,
    pub(crate) chat_scroll: u16,
    pub(crate) cursor_pos: usize,
    pub(crate) show_help: bool,
    pub(crate) help_scroll: u16,
    pub(crate) mcp_manager: Arc<McpManager>,
    pub(crate) version: String,
    pub(crate) update_available: Option<String>,
    pub(crate) memory: Memory,
    pub(crate) total_tokens: u32,
}

impl App {
    fn new() -> Result<Self> {
        let config = Config::load()?;
        Ok(Self {
            config,
            messages: Vec::new(),
            input: String::new(),
            status_message: "Press / for commands, Enter to send chat.".to_string(),
            is_loading: false,
            chat_scroll: 0,
            cursor_pos: 0,
            show_help: false,
            help_scroll: 0,
            mcp_manager: Arc::new(McpManager::new()),
            version: env!("CARGO_PKG_VERSION").to_string(),
            update_available: None,
            memory: Memory::load().unwrap_or_default(),
            total_tokens: 0,
        })
    }

    async fn init(&mut self) -> Result<()> {
        let servers = self.config.mcp_servers.clone();
        for (name, config) in servers {
            if let Err(e) = self.mcp_manager.add_server(&name, &config).await {
                self.status_message = format!("Failed to connect to MCP server {}: {}", name, e);
            }
        }
        Ok(())
    }

    fn handle_command(&mut self) {
        if self.input.starts_with("/add") {
            let parts: Vec<&str> = self.input.split_whitespace().collect();
            if parts.len() >= 4 {
                // Add model: /add <provider> <name> <api_key> [base_url]
                let provider_str = parts[1].to_lowercase();
                let provider = match provider_str.as_str() {
                    "gemini" => config::Provider::Gemini,
                    "openai" | "groq" | "ollama" => config::Provider::OpenAICompat,
                    "anthropic" | "claude" => config::Provider::Anthropic,
                    _ => {
                        self.status_message = "Unknown provider. Use: gemini, openai, anthropic, groq, ollama".to_string();
                        self.input.clear();
                        return;
                    }
                };
                let name = parts[2].to_string();
                let api_key = parts[3].to_string();
                let base_url = parts.get(4).map(|s| s.to_string());
                self.config.add_model(provider, name.clone(), api_key, base_url);
                if let Err(e) = self.config.save() {
                    self.status_message = format!("Error saving config: {}", e);
                } else {
                    self.status_message = format!("Model {} ({}) added and selected.", name, provider_str);
                    self.config.current_model = Some(name);
                }
            } else {
                self.status_message = "Usage: /add <provider> <name> <key> [url]".to_string();
            }
        } else if self.input.starts_with("/model") {
            let parts: Vec<&str> = self.input.split_whitespace().collect();
            if parts.len() == 2 {
                // Switch model: /model <name>
                let name = parts[1].to_string();
                if self.config.models.contains_key(&name) {
                    self.config.current_model = Some(name.clone());
                    self.status_message = format!("Switched to model: {}", name);
                    let _ = self.config.save();
                } else {
                    self.status_message = format!("Model {} not found.", name);
                }
            } else {
                self.status_message = "Usage: /model <name>".to_string();
            }
        } else if self.input == "/sidebar" {
            self.config.show_sidebar = !self.config.show_sidebar;
            let _ = self.config.save();
            self.status_message = format!("Sidebar {}", if self.config.show_sidebar { "enabled" } else { "disabled" });
        } else if self.input.starts_with("/remove") {
            let parts: Vec<&str> = self.input.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[1].to_string();
                if self.config.models.remove(&name).is_some() {
                    if self.config.current_model.as_ref() == Some(&name) {
                        self.config.current_model = self.config.models.keys().next().cloned();
                    }
                    if let Err(e) = self.config.save() {
                        self.status_message = format!("Error saving config: {}", e);
                    } else {
                        self.status_message = format!("Model {} removed.", name);
                    }
                } else {
                    self.status_message = format!("Model {} not found.", name);
                }
            } else {
                self.status_message = "Usage: /remove <model_name>".to_string();
            }
        } else if self.input.starts_with("/rename") {
            let parts: Vec<&str> = self.input.split_whitespace().collect();
            if parts.len() >= 3 {
                let old_name = parts[1].to_string();
                let new_name = parts[2].to_string();
                if let Some(mut model_config) = self.config.models.remove(&old_name) {
                    model_config.name = new_name.clone();
                    self.config.models.insert(new_name.clone(), model_config);
                    if self.config.current_model.as_ref() == Some(&old_name) {
                        self.config.current_model = Some(new_name.clone());
                    }
                    if let Err(e) = self.config.save() {
                        self.status_message = format!("Error saving config: {}", e);
                    } else {
                        self.status_message = format!("Model {} renamed to {}.", old_name, new_name);
                    }
                } else {
                    self.status_message = format!("Model {} not found.", old_name);
                }
            } else {
                self.status_message = "Usage: /rename <old_name> <new_name>".to_string();
            }
        } else if self.input == "/help" {
            self.show_help = true;
            self.status_message = "Help menu opened. Press ESC to close.".to_string();
        } else if self.input == "/clear" {
            self.messages.clear();
            self.chat_scroll = 0;
            self.status_message = "Chat history cleared.".to_string();
        } else if self.input.starts_with("/mcp") {
            let parts: Vec<&str> = self.input.split_whitespace().collect();
            if parts.len() >= 3 && parts[1] == "add" {
                let name = parts[2].to_string();
                let command = parts[3].to_string();
                let args = parts[4..].iter().map(|s| s.to_string()).collect::<Vec<_>>();
                let config = config::McpServerConfig {
                    command,
                    args,
                    env: std::collections::HashMap::new(),
                };
                self.config.mcp_servers.insert(name.clone(), config.clone());
                if let Err(e) = self.config.save() {
                    self.status_message = format!("Error saving config: {}", e);
                } else {
                    self.status_message = format!("MCP Server {} added. Connecting...", name);
                    let mcp = self.mcp_manager.clone();
                    tokio::spawn(async move {
                        let _ = mcp.add_server(&name, &config).await;
                    });
                }
            } else if parts.len() >= 2 && parts[1] == "list" {
                let servers: Vec<String> = self.config.mcp_servers.keys().cloned().collect();
                self.status_message = format!("MCP Servers: {}", servers.join(", "));
            } else {
                self.status_message = "Usage: /mcp add <name> <command> [args...] or /mcp list".to_string();
            }
        } else if self.input == "/save" {
            if let Err(e) = self.memory.save() {
                self.status_message = format!("Error saving memory: {}", e);
            } else {
                self.status_message = "Memory saved to memory.json".to_string();
            }
        } else {
            self.status_message = "Unknown command.".to_string();
        }
        self.input.clear();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new()?;
    app.init().await?;
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    app.mcp_manager.shutdown().await;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

use tokio::sync::mpsc;

enum Action {
    ApiResponse(Result<(String, crate::api::Usage)>),
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> 
where
    io::Error: From<B::Error>,
{
    let (tx, mut rx) = mpsc::channel(10);

    loop {
        terminal.draw(|f| ui::ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) => {
                        match key.code {
                            KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                return Ok(());
                            }
                            KeyCode::Enter => {
                                if app.input.starts_with('/') {
                                    app.handle_command();
                                    app.cursor_pos = 0;
                                } else if !app.input.is_empty() && !app.is_loading {
                                    let input = app.input.drain(..).collect::<String>();
                                    app.cursor_pos = 0;
                                    let model_name = app.config.current_model.clone();
                                    
                                    if let Some(name) = model_name {
                                        if let Some(model_config) = app.config.models.get(&name) {
                                            app.messages.push(crate::api::Message::new("user", &input));
                                            app.is_loading = true;
                                            app.status_message = format!("Waiting for {}...", name);
                                            app.chat_scroll = 0; // Scroll to bottom
                                            
                                            let api_config = model_config.clone();
                                            let mut messages = app.messages.clone();
                                            let tx = tx.clone();
                                            let mcp = app.mcp_manager.clone();
                                            
                                            tokio::spawn(async move {
                                                let client = ApiClient::new();
                                                loop {
                                                    let tools = mcp.list_tools().await.unwrap_or_default();
                                                    let res = client.send_chat_completion(&api_config, messages.clone(), tools).await;
                                                    
                                                    match res {
                                                        Ok(crate::api::ApiResult::ToolCall(assistant_msg, tool_name, tool_args, usage)) => {
                                                            messages.push(assistant_msg.clone());
                                                            let _ = tx.send(Action::ApiResponse(Ok((format!("Calling tool: {}...", tool_name), usage)))).await;
                                                            
                                                            match mcp.call_tool(&tool_name, tool_args).await {
                                                                Ok(result) => {
                                                                    let mut tool_output = String::new();
                                                                    for content in result.content {
                                                                        match &*content {
                                                                            rmcp::model::RawContent::Text(t) => {
                                                                                tool_output.push_str(&t.text);
                                                                            }
                                                                            _ => {}
                                                                        }
                                                                    }
                                                                    
                                                                    let tc_id = assistant_msg.tool_calls.as_ref()
                                                                        .and_then(|calls| calls.first())
                                                                        .map(|tc| tc.id.clone())
                                                                        .unwrap_or_else(|| "unknown".to_string());

                                                                    messages.push(crate::api::Message::new_tool_response(
                                                                        &tool_name,
                                                                        &tc_id,
                                                                        &tool_output,
                                                                    ));
                                                                }
                                                                Err(e) => {
                                                                    let _ = tx.send(Action::ApiResponse(Err(anyhow::anyhow!("Tool call failed: {}", e)))).await;
                                                                    break;
                                                                }
                                                            }
                                                        }
                                                        Ok(crate::api::ApiResult::Text(text, usage)) => {
                                                            let _ = tx.send(Action::ApiResponse(Ok((text, usage)))).await;
                                                            break;
                                                        }
                                                        Err(e) => {
                                                            let _ = tx.send(Action::ApiResponse(Err(e))).await;
                                                            break;
                                                        }
                                                    }
                                                }
                                            });
                                        }
                                    } else {
                                        app.status_message = "No model selected. Use /model command.".to_string();
                                    }
                                }
                            }
                            KeyCode::Left => {
                                if app.cursor_pos > 0 {
                                    app.cursor_pos -= 1;
                                }
                            }
                            KeyCode::Right => {
                                if app.cursor_pos < app.input.len() {
                                    app.cursor_pos += 1;
                                }
                            }
                            KeyCode::Home => {
                                app.cursor_pos = 0;
                            }
                            KeyCode::End => {
                                app.cursor_pos = app.input.len();
                            }
                            KeyCode::Up => {
                                if app.show_help {
                                    app.help_scroll = app.help_scroll.saturating_add(1);
                                } else {
                                    app.chat_scroll = app.chat_scroll.saturating_add(1);
                                }
                            }
                            KeyCode::Down => {
                                if app.show_help {
                                    app.help_scroll = app.help_scroll.saturating_sub(1);
                                } else {
                                    app.chat_scroll = app.chat_scroll.saturating_sub(1);
                                }
                            }
                            KeyCode::PageUp => {
                                if app.show_help {
                                    app.help_scroll = app.help_scroll.saturating_add(10);
                                } else {
                                    app.chat_scroll = app.chat_scroll.saturating_add(10);
                                }
                            }
                            KeyCode::PageDown => {
                                if app.show_help {
                                    app.help_scroll = app.help_scroll.saturating_sub(10);
                                } else {
                                    app.chat_scroll = app.chat_scroll.saturating_sub(10);
                                }
                            }
                            KeyCode::Char(c) => {
                                app.input.insert(app.cursor_pos, c);
                                app.cursor_pos += 1;
                                app.chat_scroll = 0; // Reset scroll on activity
                            }
                            KeyCode::Backspace => {
                                if app.cursor_pos > 0 {
                                    app.input.remove(app.cursor_pos - 1);
                                    app.cursor_pos -= 1;
                                }
                            }
                            KeyCode::Delete => {
                                if app.cursor_pos < app.input.len() {
                                    app.input.remove(app.cursor_pos);
                                }
                            }
                            KeyCode::Esc => {
                                if app.show_help {
                                    app.show_help = false;
                                    app.status_message = "Help menu closed.".to_string();
                                } else {
                                    return Ok(());
                                }
                            }
                            _ => {}
                        }
                    }
                    Event::Mouse(mouse) => {
                        let size = terminal.size()?;
                        let chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(9),  // Banner
                                Constraint::Min(0),     // Body
                                Constraint::Length(1),  // Shortcuts
                                Constraint::Length(1),  // Separator
                                Constraint::Length(1),  // Edit hint
                                Constraint::Length(3),  // Input
                                Constraint::Length(1),  // Footer
                            ])
                            .split(size.into());
                        
                        let body_chunks = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
                            .split(chunks[1]);
                        
                        let sidebar = body_chunks[0];
                        let chat_area = body_chunks[1];

                        match mouse.kind {
                            event::MouseEventKind::Down(event::MouseButton::Left) => {
                                if mouse.column > sidebar.x && mouse.column < sidebar.x + sidebar.width - 1
                                    && mouse.row > sidebar.y && mouse.row < sidebar.y + sidebar.height - 1 
                                {
                                    let clicked_row = (mouse.row - sidebar.y - 1) as usize;
                                    let mut model_names: Vec<&String> = app.config.models.keys().collect();
                                    model_names.sort();
                                    
                                    if let Some(&name) = model_names.get(clicked_row) {
                                        app.config.current_model = Some(name.clone());
                                        app.status_message = format!("Switched to model: {}", name);
                                        app.chat_scroll = 0;
                                    }
                                }
                            }
                            event::MouseEventKind::ScrollUp => {
                                if mouse.column > chat_area.x && mouse.column < chat_area.x + chat_area.width - 1
                                    && mouse.row > chat_area.y && mouse.row < chat_area.y + chat_area.height - 1 
                                {
                                    if app.show_help {
                                        app.help_scroll = app.help_scroll.saturating_add(3);
                                    } else {
                                        app.chat_scroll = app.chat_scroll.saturating_add(3);
                                    }
                                }
                            }
                            event::MouseEventKind::ScrollDown => {
                                if mouse.column > chat_area.x && mouse.column < chat_area.x + chat_area.width - 1
                                    && mouse.row > chat_area.y && mouse.row < chat_area.y + chat_area.height - 1 
                                {
                                    if app.show_help {
                                        app.help_scroll = app.help_scroll.saturating_sub(3);
                                    } else {
                                        app.chat_scroll = app.chat_scroll.saturating_sub(3);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
        }

        // Check for actions
        if let Ok(action) = rx.try_recv() {
            match action {
                Action::ApiResponse(res) => {
                    app.is_loading = false;
                    match res {
                        Ok((response, usage)) => {
                            app.total_tokens += usage.total_tokens;
                            let assistant_msg = Message::new("assistant", &response);
                            app.messages.push(assistant_msg.clone());
                            
                            // Save to memory
                            app.memory.add_interaction(vec![
                                app.messages[app.messages.len()-2].clone(), // Previous user msg
                                assistant_msg,
                            ]);
                            let _ = app.memory.save();

                            app.status_message = format!("Response received. Usage: {} tokens", usage.total_tokens);
                            app.chat_scroll = 0; // Scroll to bottom
                        }
                        Err(e) => {
                            app.status_message = format!("Error: {}", e);
                        }
                    }
                }
            }
        }
    }
}

