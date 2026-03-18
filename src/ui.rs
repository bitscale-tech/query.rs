use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use termimad::MadSkin;
use ansi_to_tui::IntoText;
use crate::App;

const BANNER: &str = r#"
 ░████████ ░██    ░██  ░███████  ░██░████ ░██    ░██     ░██░████  ░███████  
 ░██    ░██ ░██    ░██ ░██    ░██ ░███     ░██    ░██     ░███     ░██        
 ░██    ░██ ░██    ░██ ░█████████ ░██      ░██    ░██     ░██       ░███████  
 ░██   ░███ ░██   ░███ ░██        ░██      ░██   ░███     ░██             ░██ 
  ░█████░██  ░█████░██  ░███████  ░██       ░█████░██ ░██ ░██       ░███████  
        ░██                                       ░██                         
        ░██                                 ░███████                          
"#;

pub fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    // Main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),  // Banner + Header tips
            Constraint::Min(0),     // Chat History
            Constraint::Length(1),  // Shortcuts hint line
            Constraint::Length(1),  // Separator ───
            Constraint::Length(1),  // Edit hint (shift+tab)
            Constraint::Length(3),  // Input area (with ▀/▄ bars)
            Constraint::Length(1),  // Footer
        ])
        .split(size);

    // 0. Header (Banner + Sidebar/Tips)
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(30)])
        .split(chunks[0]);

    // Banner
    f.render_widget(
        Paragraph::new(BANNER).style(Style::default().fg(Color::Cyan)),
        header_chunks[0],
    );
    // Dynamic Update Box
    if let Some(version) = &app.update_available {
        let update_box = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(Color::Yellow));
        
        let update_msg = Paragraph::new(format!("query.rs update available! {} to {}", app.version, version))
            .block(update_box)
            .alignment(ratatui::layout::Alignment::Center);
        
        f.render_widget(update_msg, header_chunks[1]);
    }

    // 1. Chat History Area
    // Sidebar + Chat layout
    let body_chunks = if app.config.show_sidebar {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(chunks[1])
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(0), Constraint::Percentage(100)])
            .split(chunks[1])
    };

    // Sidebar (Models)
    let mut model_names: Vec<&String> = app.config.models.keys().collect();
    model_names.sort();
    let models: Vec<ListItem> = model_names
        .iter()
        .map(|&name| {
            let style = if app.config.current_model.as_ref() == Some(name) {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(name.clone()).style(style)
        })
        .collect();

    if app.config.show_sidebar {
        f.render_widget(
            List::new(models)
                .block(Block::default().title(" Models ").borders(Borders::ALL))
                .highlight_style(Style::default().bg(Color::DarkGray)),
            body_chunks[0],
        );
    }

    // Chat
    if app.show_help {
        draw_help(f, body_chunks[1], app);
    } else {
        draw_chat(f, body_chunks[1], app);
    }

    // 2. Shortcuts hint
    let shortcut_hint = Paragraph::new("? for shortcuts")
        .alignment(ratatui::layout::Alignment::Right)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(shortcut_hint, chunks[2]);

    // 3. Separator ───
    let separator = Paragraph::new("─".repeat(size.width as usize))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(separator, chunks[3]);

    // 4. Edit hint
    let edit_hint = Paragraph::new(" shift+tab to accept edits")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(edit_hint, chunks[4]);

    // 5. Input Area
    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Top bar ▀▀▀
            Constraint::Length(1), // Message > ...
            Constraint::Length(1), // Bottom bar ▄▄▄
        ])
        .split(chunks[5]);

    f.render_widget(
        Paragraph::new("▀".repeat(size.width as usize)).style(Style::default().fg(Color::DarkGray)),
        input_chunks[0],
    );

    let input_label = " > ";
    let input_text = if app.input.is_empty() && !app.is_loading {
        "  Type your message or @path/to/file"
    } else {
        &app.input
    };
    
    let input_style = if app.input.is_empty() {
        Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
    } else {
        Style::default()
    };

    let input_p = Paragraph::new(Line::from(vec![
        Span::raw(input_label),
        Span::styled(input_text, input_style),
    ]));
    f.render_widget(input_p, input_chunks[1]);

    f.render_widget(
        Paragraph::new("▄".repeat(size.width as usize)).style(Style::default().fg(Color::DarkGray)),
        input_chunks[2],
    );

    // Cursor
    if !app.is_loading {
        f.set_cursor_position((
            input_chunks[1].x + input_label.len() as u16 + app.cursor_pos as u16,
            input_chunks[1].y,
        ));
    }

    // 6. Footer
    let model_name = app.config.current_model.as_ref().cloned().unwrap_or_else(|| "None".to_string());
    let cwd = std::env::current_dir().unwrap_or_default().display().to_string();
    let footer_left = format!(" {} [Tokens: {}]", cwd, app.total_tokens);
    let footer_right = format!("/model switch ({})", model_name);
    
    let footer_line = Line::from(vec![
        Span::styled(&footer_left, Style::default().fg(Color::DarkGray)),
        Span::raw(" ".repeat(size.width.saturating_sub(footer_left.len() as u16).saturating_sub(footer_right.len() as u16) as usize)),
        Span::styled(footer_right, Style::default().fg(Color::DarkGray)),
    ]);
    
    f.render_widget(Paragraph::new(footer_line), chunks[6]);
}

fn draw_chat(f: &mut Frame, area: Rect, app: &App) {
    let mut full_text = String::new();
    for msg in &app.messages {
        let role_display = if msg.role == "user" { "**You**" } else { "**AI**" };
        full_text.push_str(&format!("{}: {}\n\n---\n\n", role_display, msg.content_text()));
    }

    let skin = MadSkin::default();
    let wrapped = skin.text(&full_text, Some(area.width as usize));
    let total_lines = wrapped.lines.len();
    
    let scroll = if app.chat_scroll == 0 {
        total_lines.saturating_sub(area.height as usize) as u16
    } else {
        (total_lines.saturating_sub(area.height as usize).saturating_sub(app.chat_scroll as usize)) as u16
    };

    let tui_text = wrapped.to_string().into_text().unwrap_or_default();
    let paragraph = Paragraph::new(tui_text)
        .block(Block::default().borders(Borders::ALL).title(" Chat "))
        .scroll((scroll, 0));
    
    f.render_widget(paragraph, area);
}

fn draw_help(f: &mut Frame, area: Rect, app: &App) {
    let help_text = vec![
        "Keyboard Shortcuts:",
        " - Enter       : Send Message / Run Command",
        " - Tab/S-Tab   : Cycle Focus (Sidebar / Input)",
        " - Esc         : Close Help / Exit App",
        " - Up/Down     : Scroll Chat history",
        " - PgUp/PgDn   : Fast Scroll",
        " - Mouse Wheel : Scroll Chat",
        " - Mouse Click : Select Model in Sidebar",
        "",
        "Commands:",
        " /add <provider> <name> <key> [url] : Add AI model",
        " /model <name>                       : Switch to existing model",
        " /sidebar                            : Toggle sidebar visibility",
        " /remove <name>                       : Remove model from config",
        " /rename <old> <new>                   : Rename a model",
        " /mcp add <name> <cmd> [args]          : Connect an MCP tool server",
        " /mcp list                           : List connected MCP servers",
        " /clear                              : Clear current chat history",
        " /save                               : Force save session to memory.json",
        " /help                               : Toggle this menu",
        "",
        "Environment:",
        " query-rs looks for a .env file in the current directory.",
        " You can store API keys there (e.g., GEMINI_API_KEY=...).",
        "",
        "Session Info:",
        &format!(" - Version: v{}", app.version),
        &format!(" - Total Session Tokens: {}", app.total_tokens),
    ].join("\n");
    
    let help_p = Paragraph::new(help_text)
        .block(Block::default()
            .title(" HELP & DOCUMENTATION ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)));
    
    f.render_widget(help_p, area);
}
