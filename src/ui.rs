use crate::App;
use ansi_to_tui::IntoText;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};
use termimad::MadSkin;

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

    // Determine header height: banner (9) + optional update box (3)
    let update_visible = app.update_available.is_some();
    let header_height: u16 = if update_visible { 9 + 3 } else { 9 };

    // Main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height), // Banner + optional update box
            Constraint::Min(0),                // Chat area (full width)
            Constraint::Length(1),             // "? for shortcuts"
            Constraint::Length(1),             // Separator ─────
            Constraint::Length(1),             // "shift+tab to accept edits"
            Constraint::Length(3),             // Input area (▀ / > / ▄)
            Constraint::Length(1),             // Footer
        ])
        .split(size);

    // --- 0. Header ---
    // Stack banner and (optionally) the update box vertically
    let banner_area;
    let update_area_opt;
    if update_visible {
        let header_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(9), Constraint::Length(3)])
            .split(chunks[0]);
        banner_area = header_chunks[0];
        update_area_opt = Some(header_chunks[1]);
    } else {
        banner_area = chunks[0];
        update_area_opt = None;
    }

    f.render_widget(
        Paragraph::new(BANNER).style(Style::default().fg(Color::Cyan)),
        banner_area,
    );

    if let (Some(update_area), Some(version)) = (update_area_opt, &app.update_available) {
        let update_box = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Yellow));
        let update_msg = Paragraph::new(format!(
            "query.rs update available! {} → {}",
            app.version, version
        ))
        .block(update_box)
        .alignment(Alignment::Center);
        f.render_widget(update_msg, update_area);
    }

    // --- 1. Chat (full width) ---
    if app.show_help {
        draw_help(f, chunks[1], app);
    } else {
        draw_chat(f, chunks[1], app);
    }

    // --- 2. Shortcuts hint ---
    f.render_widget(
        Paragraph::new("? for shortcuts")
            .alignment(Alignment::Right)
            .style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );

    // --- 3. Separator ---
    f.render_widget(
        Paragraph::new("─".repeat(size.width as usize)).style(Style::default().fg(Color::DarkGray)),
        chunks[3],
    );

    // --- 4. Edit hint ---
    f.render_widget(
        Paragraph::new(" shift+tab to accept edits").style(Style::default().fg(Color::DarkGray)),
        chunks[4],
    );

    // --- 5. Input area ---
    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // ▀▀▀
            Constraint::Length(1), // > msg
            Constraint::Length(1), // ▄▄▄
        ])
        .split(chunks[5]);

    f.render_widget(
        Paragraph::new("▀".repeat(size.width as usize)).style(Style::default().fg(Color::DarkGray)),
        input_chunks[0],
    );

    let input_label = " > ";
    let (input_text, input_style) = if app.input.is_empty() && !app.is_loading {
        (
            "  Type your message or @path/to/file",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )
    } else {
        (app.input.as_str(), Style::default())
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(input_label),
            Span::styled(input_text, input_style),
        ])),
        input_chunks[1],
    );

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

    // --- 6. Footer ---
    let model_name = app.config.current_model.as_deref().unwrap_or("None");
    let footer_left = " ~";
    let footer_center = "no sandbox (see /docs)";
    let footer_right = format!("/model Auto ({})", model_name);

    let left_len = footer_left.len() as u16;
    let center_len = footer_center.len() as u16;
    let right_len = footer_right.len() as u16;
    let total = size.width;
    let padding_left = (total / 2).saturating_sub(left_len + center_len / 2);
    let padding_right = total
        .saturating_sub(left_len)
        .saturating_sub(padding_left)
        .saturating_sub(center_len)
        .saturating_sub(right_len);

    let footer_line = Line::from(vec![
        Span::styled(footer_left, Style::default().fg(Color::DarkGray)),
        Span::raw(" ".repeat(padding_left as usize)),
        Span::styled(footer_center, Style::default().fg(Color::DarkGray)),
        Span::raw(" ".repeat(padding_right as usize)),
        Span::styled(footer_right, Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(footer_line), chunks[6]);

    // --- Model Menu Overlay ---
    if app.show_model_menu {
        draw_model_menu(f, app);
    }
}

/// Draw a centered popup listing all models for selection.
fn draw_model_menu(f: &mut Frame, app: &App) {
    let mut model_names: Vec<&String> = app.config.models.keys().collect();
    model_names.sort();

    let popup_height = (model_names.len() as u16 + 4).min(f.area().height - 4);
    let popup_width = (model_names.iter().map(|n| n.len()).max().unwrap_or(20) as u16 + 6)
        .max(40)
        .min(f.area().width - 4);

    let area = centered_rect(popup_width, popup_height, f.area());

    f.render_widget(Clear, area);

    let items: Vec<ListItem> = model_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let is_current = app.config.current_model.as_deref() == Some(name.as_str());
            let is_selected = i == app.model_menu_selected;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if is_current {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let marker = if is_current { " ✓ " } else { "   " };
            ListItem::new(format!("{}{}", marker, name)).style(style)
        })
        .collect();

    let block = Block::default()
        .title(" Select Model (↑↓ navigate, Enter select, Esc close) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

/// Returns a centered rect of given width and height within `r`.
fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(r.width),
        height: height.min(r.height),
    }
}

fn draw_chat(f: &mut Frame, area: Rect, app: &App) {
    let mut full_text = String::new();
    for msg in &app.messages {
        let role_display = if msg.role == "user" {
            "**You**"
        } else {
            "**AI**"
        };
        full_text.push_str(&format!(
            "{}: {}\n\n---\n\n",
            role_display,
            msg.content_text()
        ));
    }

    let skin = MadSkin::default();
    let wrapped = skin.text(&full_text, Some(area.width as usize));
    let total_lines = wrapped.lines.len();

    let scroll = if app.chat_scroll == 0 {
        total_lines.saturating_sub(area.height as usize) as u16
    } else {
        total_lines
            .saturating_sub(area.height as usize)
            .saturating_sub(app.chat_scroll as usize) as u16
    };

    let tui_text = wrapped.to_string().into_text().unwrap_or_default();
    let paragraph = Paragraph::new(tui_text)
        .block(Block::default().borders(Borders::ALL).title(" Chat "))
        .scroll((scroll, 0));

    f.render_widget(paragraph, area);
}

fn draw_help(f: &mut Frame, area: Rect, app: &App) {
    let help_lines = vec![
        "Keyboard Shortcuts:".to_string(),
        " - Enter       : Send Message / Run Command".to_string(),
        " - Esc         : Close Help / Model Menu / Exit App".to_string(),
        " - Up/Down     : Scroll Chat history".to_string(),
        " - PgUp/PgDn   : Fast Scroll".to_string(),
        " - Mouse Wheel : Scroll Chat".to_string(),
        "".to_string(),
        "Commands:".to_string(),
        " /add <provider> <name> <key> [url]  : Add AI model".to_string(),
        " /model                              : Open model selection menu".to_string(),
        " /model <name>                       : Switch to model directly".to_string(),
        " /remove <name>                      : Remove model from config".to_string(),
        " /rename <old> <new>                 : Rename a model".to_string(),
        " /mcp add <name> <cmd> [args]        : Connect an MCP tool server".to_string(),
        " /mcp list                           : List connected MCP servers".to_string(),
        " /clear                              : Clear current chat history".to_string(),
        " /save                               : Force save session to memory".to_string(),
        " /help                               : Toggle this menu".to_string(),
        "".to_string(),
        "Environment:".to_string(),
        " query-rs loads a .env file in the current directory.".to_string(),
        " Store API keys there (e.g., GEMINI_API_KEY=...).".to_string(),
        "".to_string(),
        "Session Info:".to_string(),
        format!(" - Version: v{}", app.version),
        format!(" - Total Session Tokens: {}", app.total_tokens),
        format!(
            " - Working Directory: {}",
            std::env::current_dir().unwrap_or_default().display()
        ),
    ];

    let help_text = help_lines.join("\n");
    let help_p = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" HELP & DOCUMENTATION ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .scroll((app.help_scroll, 0));

    f.render_widget(help_p, area);
}
