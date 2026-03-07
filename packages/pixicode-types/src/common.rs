//! Common types shared across all pixicode applications

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Pixicode version information.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VersionInfo {
    /// Version string (e.g., "0.1.0")
    pub version: String,
    /// Build timestamp
    pub build_date: Option<String>,
    /// Git commit hash
    pub git_hash: Option<String>,
    /// Target triple
    pub target: Option<String>,
}

/// Application state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AppState {
    /// Application is starting up
    Initializing,
    /// Application is ready
    Ready,
    /// Application is processing
    Busy,
    /// Application is shutting down
    ShuttingDown,
}

/// Log level for filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

/// Log entry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LogEntry {
    /// Timestamp
    pub timestamp: String,
    /// Log level
    pub level: LogLevel,
    /// Target/module
    pub target: String,
    /// Message
    pub message: String,
    /// Additional fields
    pub fields: Option<serde_json::Value>,
}

/// Progress information.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct Progress {
    /// Current progress (0.0 - 1.0)
    pub current: f32,
    /// Total progress target
    pub total: f32,
    /// Progress message
    pub message: Option<String>,
    /// Whether progress is indeterminate
    pub indeterminate: bool,
}

impl Progress {
    pub fn new(current: f32, total: f32) -> Self {
        Self {
            current,
            total,
            message: None,
            indeterminate: false,
        }
    }

    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }

    pub fn percentage(&self) -> f32 {
        if self.total == 0.0 {
            0.0
        } else {
            (self.current / self.total * 100.0).min(100.0)
        }
    }
}

/// Result with progress.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProgressResult<T> {
    /// The result value
    pub result: Option<T>,
    /// Progress information
    pub progress: Progress,
    /// Error message if failed
    pub error: Option<String>,
}

impl<T> ProgressResult<T> {
    pub fn success(result: T) -> Self {
        Self {
            result: Some(result),
            progress: Progress {
                current: 1.0,
                total: 1.0,
                message: Some("Completed".to_string()),
                indeterminate: false,
            },
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            result: None,
            progress: Progress::default(),
            error: Some(error),
        }
    }

    pub fn with_progress(mut self, progress: Progress) -> Self {
        self.progress = progress;
        self
    }
}

/// Key binding.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KeyBinding {
    /// Key combination (e.g., "Ctrl+Q", "Enter")
    pub key: String,
    /// Action to perform
    pub action: String,
    /// Description
    pub description: Option<String>,
    /// Whether binding is active
    pub active: bool,
}

/// Theme color.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ThemeColor {
    /// Color name
    pub name: String,
    /// Hex color value
    pub hex: String,
    /// RGB values
    pub rgb: Option<(u8, u8, u8)>,
}

/// UI Theme.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UiTheme {
    /// Theme name
    pub name: String,
    /// Whether it's a dark theme
    pub dark: bool,
    /// Primary color
    pub primary: ThemeColor,
    /// Background color
    pub background: ThemeColor,
    /// Foreground color
    pub foreground: ThemeColor,
    /// Accent color
    pub accent: Option<ThemeColor>,
}

/// Notification type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    Info,
    Success,
    Warning,
    Error,
}

/// Notification.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Notification {
    /// Unique ID
    pub id: String,
    /// Notification type
    #[serde(rename = "type")]
    pub notification_type: NotificationType,
    /// Title
    pub title: String,
    /// Message
    pub message: String,
    /// Timestamp
    pub timestamp: String,
    /// Whether notification has been read
    pub read: bool,
    /// Actions available
    pub actions: Vec<NotificationAction>,
}

/// Notification action.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NotificationAction {
    /// Action label
    pub label: String,
    /// Action command
    pub command: String,
}

impl Notification {
    pub fn info(title: String, message: String) -> Self {
        Self {
            id: ulid::Ulid::new().to_string(),
            notification_type: NotificationType::Info,
            title,
            message,
            timestamp: chrono::Utc::now().to_rfc3339(),
            read: false,
            actions: Vec::new(),
        }
    }

    pub fn error(title: String, message: String) -> Self {
        Self {
            id: ulid::Ulid::new().to_string(),
            notification_type: NotificationType::Error,
            title,
            message,
            timestamp: chrono::Utc::now().to_rfc3339(),
            read: false,
            actions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress() {
        let progress = Progress::new(50.0, 100.0);
        assert_eq!(progress.percentage(), 50.0);
    }

    #[test]
    fn test_notification() {
        let notification = Notification::info(
            "Test".to_string(),
            "Test message".to_string()
        );
        assert_eq!(notification.notification_type, NotificationType::Info);
        assert!(!notification.read);
    }
}
