//! ACP types

use serde::{Deserialize, Serialize};

/// ACP Task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpTask {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
    pub progress: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}
