//! Application state

use crossterm::event;
use pixicode_types::session::SessionSummary;
use pixicode_types::common::Notification;
use ratatui::style::Color;

/// Main application state.
pub struct App {
    /// Whether app should quit
    pub should_quit: bool,
    /// Current view
    pub view: View,
    /// Input buffer
    pub input: String,
    /// Input cursor position
    pub cursor_position: usize,
    /// Sessions list
    pub sessions: Vec<SessionSummary>,
    /// Selected session index
    pub selected_session: Option<usize>,
    /// Chat history
    pub messages: Vec<ChatMessage>,
    /// Notifications
    pub notifications: Vec<Notification>,
    /// Status message
    pub status: Option<StatusMessage>,
    /// Current theme
    pub theme: Theme,
    /// Theme name (for switching)
    pub theme_name: ThemeName,
}

/// Current view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Session list
    Sessions,
    /// Chat view
    Chat,
    /// Settings
    Settings,
    /// Help
    Help,
}

/// Chat message for display.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

/// Status message.
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub message: String,
    pub level: StatusLevel,
}

/// Status level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Theme name for switching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeName {
    #[default]
    Dark,
    Light,
}

/// Application theme (keybindings and input handling in input.rs).
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub primary: Color,
    pub secondary: Color,
    pub error: Color,
    pub success: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            background: Color::Rgb(30, 30, 46),
            foreground: Color::Rgb(205, 214, 244),
            primary: Color::Rgb(137, 180, 250),
            secondary: Color::Rgb(166, 227, 161),
            error: Color::Rgb(243, 139, 168),
            success: Color::Rgb(166, 227, 161),
        }
    }

    pub fn light() -> Self {
        Self {
            background: Color::Rgb(239, 241, 245),
            foreground: Color::Rgb(76, 79, 105),
            primary: Color::Rgb(30, 102, 245),
            secondary: Color::Rgb(64, 160, 43),
            error: Color::Rgb(210, 15, 57),
            success: Color::Rgb(64, 160, 43),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_quit: false,
            view: View::Sessions,
            input: String::new(),
            cursor_position: 0,
            sessions: Vec::new(),
            selected_session: None,
            messages: Vec::new(),
            notifications: Vec::new(),
            status: None,
            theme: Theme::default(),
            theme_name: ThemeName::default(),
        }
    }
}

fn apply_theme(theme_name: ThemeName) -> Theme {
    match theme_name {
        ThemeName::Dark => Theme::dark(),
        ThemeName::Light => Theme::light(),
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_key(&mut self, key: event::KeyEvent) -> bool {
        match key.code {
            event::KeyCode::Char('q') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return true;
            }
            event::KeyCode::Char('t') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.theme_name = match self.theme_name {
                    ThemeName::Dark => ThemeName::Light,
                    ThemeName::Light => ThemeName::Dark,
                };
                self.theme = apply_theme(self.theme_name);
                self.set_status("Theme switched", StatusLevel::Info);
            }
            event::KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                self.input.clear();
                self.cursor_position = 0;
            }
            event::KeyCode::Char(c) => {
                self.input.insert(self.cursor_position, c);
                self.cursor_position += 1;
            }
            event::KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.input.remove(self.cursor_position - 1);
                    self.cursor_position -= 1;
                }
            }
            event::KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            event::KeyCode::Right => {
                if self.cursor_position < self.input.len() {
                    self.cursor_position += 1;
                }
            }
            event::KeyCode::Enter => {
                self.handle_enter();
            }
            event::KeyCode::Tab => {
                self.cycle_view();
            }
            event::KeyCode::Esc => {
                self.input.clear();
                self.cursor_position = 0;
            }
            _ => {}
        }
        false
    }

    fn handle_enter(&mut self) {
        match self.view {
            View::Sessions => {
                if let Some(idx) = self.selected_session {
                    if idx < self.sessions.len() {
                        self.view = View::Chat;
                        self.set_status("Opened session", StatusLevel::Info);
                    }
                }
            }
            View::Chat => {
                if !self.input.is_empty() {
                    self.messages.push(ChatMessage {
                        role: "user".to_string(),
                        content: self.input.clone(),
                        timestamp: chrono::Local::now().format("%H:%M").to_string(),
                    });
                    self.input.clear();
                    self.cursor_position = 0;
                    // TODO: Send to AI and get response
                }
            }
            _ => {}
        }
    }

    fn cycle_view(&mut self) {
        self.view = match self.view {
            View::Sessions => View::Chat,
            View::Chat => View::Settings,
            View::Settings => View::Help,
            View::Help => View::Sessions,
        };
    }

    pub fn set_status(&mut self, message: &str, level: StatusLevel) {
        self.status = Some(StatusMessage {
            message: message.to_string(),
            level,
        });
    }

    pub fn add_notification(&mut self, notification: Notification) {
        self.notifications.push(notification);
        // Keep only last 5 notifications
        if self.notifications.len() > 5 {
            self.notifications.remove(0);
        }
    }
}
