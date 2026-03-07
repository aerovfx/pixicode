//! Input handling and keybinding definitions.
//!
//! Keybindings are applied in app.rs; this module documents and centralises them.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Quit: Ctrl+Q
pub fn is_quit(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL)
}

/// Clear input: Ctrl+C
pub fn is_clear_input(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

/// Toggle theme: Ctrl+T
pub fn is_toggle_theme(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::CONTROL)
}

/// Tab: switch view
pub fn is_switch_view(key: &KeyEvent) -> bool {
    key.code == KeyCode::Tab
}

/// Enter: submit / open
pub fn is_submit(key: &KeyEvent) -> bool {
    key.code == KeyCode::Enter
}

/// Esc: clear input
pub fn is_escape(key: &KeyEvent) -> bool {
    key.code == KeyCode::Esc
}
