mod config;
mod api;

use anyhow::Result;
use api::{ApiClient, Message};
use config::Config;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::time::Duration;

struct App {
    config: Config,
    messages: Vec<Message>,
    input: String,
    status_message: String,
    is_loading: bool,
    chat_scroll: u16,
    cursor_pos: usize,
    show_help: bool,
    help_scroll: u16,
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
        })
    }

    fn handle_command(&mut self) {
        if self.input.starts_with("/model") {
            let parts: Vec<&str> = self.input.split_whitespace().collect();
            if parts.len() >= 4 {
                let provider_str = parts[1].to_lowercase();
                let provider = match provider_str.as_str() {
                    "gemini" => config::Provider::Gemini,
                    "openai" | "groq" | "ollama" => config::Provider::OpenAICompat,
                    _ => {
                        self.status_message = "Unknown provider. Use: gemini, openai, groq, ollama".to_string();
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
                self.status_message = "Usage: /model <provider> <name> <api_key> [base_url]".to_string();
            }
        } else if self.input.starts_with("/switch") {
            let parts: Vec<&str> = self.input.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[1].to_string();
                if self.config.models.contains_key(&name) {
                    self.config.current_model = Some(name.clone());
                    self.status_message = format!("Switched to model: {}", name);
                } else {
                    self.status_message = format!("Model {} not found.", name);
                }
            } else {
                self.status_message = "Usage: /switch <model_name>".to_string();
            }
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
        } else {
            self.status_message = "Unknown command.".to_string();
        }
        self.input.clear();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new()?;
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

use tokio::sync::mpsc;

enum Action {
    ApiResponse(Result<String>),
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> 
where
    io::Error: From<B::Error>,
{
    let (tx, mut rx) = mpsc::channel(10);

    loop {
        terminal.draw(|f| ui(f, app))?;

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
                                            app.messages.push(Message {
                                                role: "user".to_string(),
                                                content: input.clone(),
                                            });
                                            app.is_loading = true;
                                            app.status_message = format!("Waiting for {}...", name);
                                            app.chat_scroll = 0; // Scroll to bottom
                                            
                                            let api_config = model_config.clone();
                                            let messages = app.messages.clone();
                                            let tx = tx.clone();
                                            
                                            tokio::spawn(async move {
                                                let client = ApiClient::new();
                                                let res = client.send_chat_completion(&api_config, messages).await;
                                                let _ = tx.send(Action::ApiResponse(res)).await;
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
                        let main_chunks = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Length(3),
                                Constraint::Min(0),
                                Constraint::Length(3),
                                Constraint::Length(1),
                            ].as_ref())
                            .split(size.into());
                        
                        let body_chunks = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
                            .split(main_chunks[1]);
                        
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
                        Ok(response) => {
                            app.messages.push(Message {
                                role: "assistant".to_string(),
                                content: response,
                            });
                            app.status_message = "Response received.".to_string();
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

fn ui(f: &mut Frame, app: &App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(f.area());

    // Status / Title bar
    let current_model = app.config.current_model.as_deref().unwrap_or("None");
    let title = Paragraph::new(format!("Current Model: {}", current_model))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title("Query.rs"));
    f.render_widget(title, main_chunks[0]);

    // Body with Sidebar and Chat
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
        .split(main_chunks[1]);

    // Sidebar: Models
    let mut model_names: Vec<&String> = app.config.models.keys().collect();
    model_names.sort();
    let models: Vec<ListItem> = model_names.iter()
        .map(|&m| {
            let style = if Some(m.as_str()) == app.config.current_model.as_deref() {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            ListItem::new(m.as_str()).style(style)
        })
        .collect();
    let model_list = List::new(models)
        .block(Block::default().borders(Borders::ALL).title("Models"))
        .style(Style::default().fg(Color::White));
    f.render_widget(model_list, body_chunks[0]);

    // Chat history or Help
    let chat_area = body_chunks[1];
    if app.show_help {
        let help_text = "### Commands\n\n- `/model <provider> <name> <api_key> [base_url]` - Add a new model.\n  - Providers: `openai`, `gemini`, `groq`, `ollama` \n- `/switch <model_name>` - Switch to another model.\n- `/remove <model_name>` - Remove a model from config.\n- `/rename <old> <new>` - Rename an existing model.\n- `/clear` - Clear chat history.\n- `/help` - Show help message.\n- `ESC` - Exit.\n\n### Keybindings\n\n- `Enter`: Send message\n- `Up/Down/PgUp/PgDn`: Scroll chat history\n- `Left/Right/Home/End`: Navigate input cursor\n- `Delete/Backspace`: Edit text\n\n### Interaction\n\n- **Sidebar**: Click on a model name to switch models.\n- **Chat**: Use Mouse Wheel to scroll history.\n\n## Configuration\n\nConfig is stored in `~/.config/query.rs/config.json`.";
        let help_md = tui_markdown::from_str(help_text);
        
        let help_inner_width = chat_area.width.saturating_sub(2) as usize;
        let wrapped_help = textwrap::wrap(help_text, help_inner_width);
        let total_help_lines = wrapped_help.len() as u16;
        let view_height = chat_area.height.saturating_sub(2);
        
        // Help scroll logic (from top)
        let max_help_scroll = total_help_lines.saturating_sub(view_height);
        let clamped_help_scroll = app.help_scroll.min(max_help_scroll);
        let help_scroll_y = max_help_scroll.saturating_sub(clamped_help_scroll);

        let help_para = Paragraph::new(help_md)
            .block(Block::default().borders(Borders::ALL).title("Help Menu"))
            .wrap(ratatui::widgets::Wrap { trim: false })
            .scroll((help_scroll_y, 0));
        f.render_widget(help_para, chat_area);
    } else {
        let mut full_chat_raw = String::new();
        let mut full_chat_md = String::new();
        for m in &app.messages {
            let prefix = if m.role == "user" { "You" } else { "AI" };
            full_chat_raw.push_str(&format!("{}: {}\n\n", prefix, m.content));
            full_chat_md.push_str(&format!("**{}**: {}\n\n", prefix, m.content));
        }
        
        // Calculate total lines for scrolling
        let chat_inner_width = chat_area.width.saturating_sub(2) as usize;
        let wrapped_chat = textwrap::wrap(&full_chat_raw, chat_inner_width);
        let total_chat_lines = wrapped_chat.len() as u16;
        let view_height = chat_area.height.saturating_sub(2);
        
        // Calculate scroll from top
        let max_scroll = total_chat_lines.saturating_sub(view_height);
        let clamped_chat_scroll = app.chat_scroll.min(max_scroll);
        let scroll_y = max_scroll.saturating_sub(clamped_chat_scroll);

        let chat_text = tui_markdown::from_str(&full_chat_md);
        let message_list = Paragraph::new(chat_text)
            .block(Block::default().borders(Borders::ALL).title("Chat"))
            .wrap(ratatui::widgets::Wrap { trim: false })
            .scroll((scroll_y, 0));
        f.render_widget(message_list, chat_area);
    }

    // Input box logic
    let input_area = main_chunks[2];
    let inner_width = (input_area.width as usize).saturating_sub(2);
    
    // We use a simpler cursor calculation to avoid tui/textwrap drift
    let mut cursor_x = 0;
    let mut cursor_y = 0;
    if inner_width > 0 {
        cursor_x = (app.cursor_pos % inner_width) as u16;
        cursor_y = (app.cursor_pos / inner_width) as u16;
    }

    let input_scroll = cursor_y;

    let input = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Input (Type /model for help)"))
        .wrap(ratatui::widgets::Wrap { trim: false })
        .scroll((input_scroll, 0));
    f.render_widget(input, input_area);

    // Set cursor
    f.set_cursor_position((
        input_area.x + 1 + cursor_x,
        input_area.y + 1, // Always show the current typing line at the top of the input box
    ));

    // Status bar
    let status = Paragraph::new(app.status_message.as_str())
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(status, main_chunks[3]);
}
