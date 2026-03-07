//! TUI rendering

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Stylize,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, View, StatusLevel};

/// Draw the UI.
pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Input
            Constraint::Length(1), // Status bar
        ])
        .split(f.area());

    draw_header(f, chunks[0], app);
    draw_main(f, chunks[1], app);
    draw_input(f, chunks[2], app);
    draw_status_bar(f, chunks[3], app);
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let title = match app.view {
        View::Sessions => "Sessions",
        View::Chat => "Chat",
        View::Settings => "Settings",
        View::Help => "Help",
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled("pixicode", Style::default().fg(app.theme.primary).add_modifier(Modifier::BOLD)),
        Span::raw(" - "),
        Span::raw(title),
    ]))
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(app.theme.primary)));

    f.render_widget(header, area);
}

fn draw_main(f: &mut Frame, area: Rect, app: &App) {
    match app.view {
        View::Sessions => draw_sessions(f, area, app),
        View::Chat => draw_chat(f, area, app),
        View::Settings => draw_settings(f, area, app),
        View::Help => draw_help(f, area, app),
    }
}

fn draw_sessions(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app.sessions.iter()
        .map(|s| {
            let title = s.title.as_deref().unwrap_or("Untitled");
            let subtitle = format!("{} • {} messages", s.model, s.message_count);
            ListItem::new(Line::from(vec![
                Span::styled(title, Style::default().fg(app.theme.foreground)),
                Span::raw(" - "),
                Span::styled(subtitle, Style::default().fg(app.theme.secondary)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title("Sessions").borders(Borders::ALL))
        .highlight_style(Style::default().bg(app.theme.primary).fg(app.theme.background));

    f.render_widget(list, area);
}

fn draw_chat(f: &mut Frame, area: Rect, app: &App) {
    let messages: Vec<Line> = app.messages.iter()
        .map(|m| {
            let role_style = if m.role == "user" {
                Style::default().fg(app.theme.primary)
            } else {
                Style::default().fg(app.theme.secondary)
            };
            Line::from(vec![
                Span::styled(format!("[{}] ", m.role), role_style),
                Span::raw(&m.content),
            ])
        })
        .collect();

    let chat = Paragraph::new(Text::from(messages))
        .block(Block::default().title("Chat").borders(Borders::ALL))
        .wrap(Wrap { trim: true });

    f.render_widget(chat, area);
}

fn draw_settings(f: &mut Frame, area: Rect, _app: &App) {
    let settings = Paragraph::new("Settings (not implemented)")
        .block(Block::default().title("Settings").borders(Borders::ALL));

    f.render_widget(settings, area);
}

fn draw_help(f: &mut Frame, area: Rect, app: &App) {
    let help = vec![
        Line::from("Keybindings:"),
        Line::from(""),
        Line::from(Span::styled("Ctrl+Q", Style::default().fg(app.theme.primary)).add_modifier(Modifier::BOLD)),
        Line::from("  Quit"),
        Line::from(""),
        Line::from(Span::styled("Ctrl+T", Style::default().fg(app.theme.primary)).add_modifier(Modifier::BOLD)),
        Line::from("  Toggle theme (dark/light)"),
        Line::from(""),
        Line::from(Span::styled("Tab", Style::default().fg(app.theme.primary)).add_modifier(Modifier::BOLD)),
        Line::from("  Switch view"),
        Line::from(""),
        Line::from(Span::styled("Enter", Style::default().fg(app.theme.primary)).add_modifier(Modifier::BOLD)),
        Line::from("  Send message / Open session"),
        Line::from(""),
        Line::from(Span::styled("Esc", Style::default().fg(app.theme.primary)).add_modifier(Modifier::BOLD)),
        Line::from("  Clear input"),
    ];

    let help_widget = Paragraph::new(help)
        .block(Block::default().title("Help").borders(Borders::ALL));

    f.render_widget(help_widget, area);
}

fn draw_input(f: &mut Frame, area: Rect, app: &App) {
    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().title("Input").borders(Borders::ALL).border_style(Style::default().fg(app.theme.primary)))
        .style(Style::default().fg(app.theme.foreground));

    f.render_widget(input, area);

    // Set cursor position
    f.set_cursor_position((area.x + app.cursor_position as u16 + 1, area.y + 1));
}

fn draw_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let status_text = if let Some(status) = &app.status {
        let style = match status.level {
            StatusLevel::Info => Style::default().fg(app.theme.primary),
            StatusLevel::Success => Style::default().fg(app.theme.success),
            StatusLevel::Warning => Style::default().fg(app.theme.secondary),
            StatusLevel::Error => Style::default().fg(app.theme.error),
        };
        Span::styled(&status.message, style)
    } else {
        Span::raw("Ready")
    };

    let status = Paragraph::new(Line::from(status_text));
    f.render_widget(status, area);
}
