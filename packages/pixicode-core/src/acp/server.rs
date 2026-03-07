//! ACP Server

use crate::acp::types::{AcpTask, TaskStatus};

/// ACP Server for task execution and progress reporting.
pub struct AcpServer {
    tasks: Vec<AcpTask>,
}

impl AcpServer {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn create_task(&mut self, description: String) -> AcpTask {
        let task = AcpTask {
            id: ulid::Ulid::new().to_string(),
            description,
            status: TaskStatus::Pending,
            progress: None,
        };
        self.tasks.push(task.clone());
        task
    }

    pub fn update_progress(&mut self, task_id: &str, progress: f32) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.progress = Some(progress.clamp(0.0, 100.0) / 100.0);
            if task.progress == Some(1.0) {
                task.status = TaskStatus::Completed;
            }
        }
    }
}

impl Default for AcpServer {
    fn default() -> Self {
        Self::new()
    }
}
