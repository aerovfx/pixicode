//! Todo Tool — todo list management

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::Mutex;
use std::sync::{Arc, OnceLock};

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// A single todo item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    /// Unique ID
    pub id: String,
    /// Todo text
    pub text: String,
    /// Whether completed
    pub completed: bool,
    /// Priority (low, medium, high)
    pub priority: Option<String>,
    /// Optional notes
    pub notes: Option<String>,
    /// Created timestamp
    pub created_at: Option<String>,
}

/// Todo list state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TodoList {
    /// List items
    pub items: Vec<TodoItem>,
}

// Global todo list storage (in production, use persistent storage)
static TODO_LIST: OnceLock<Arc<Mutex<TodoList>>> = OnceLock::new();

fn get_todo_list() -> Arc<Mutex<TodoList>> {
    TODO_LIST
        .get_or_init(|| Arc::new(Mutex::new(TodoList::default())))
        .clone()
}

/// Parameters for the todo tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum TodoAction {
    /// Add a new todo
    Add {
        text: String,
        priority: Option<String>,
        notes: Option<String>,
    },
    /// List todos
    List {
        /// Filter by status
        status: Option<String>,
        /// Filter by priority
        priority: Option<String>,
    },
    /// Complete a todo
    Complete {
        id: String,
    },
    /// Uncomplete a todo
    Uncomplete {
        id: String,
    },
    /// Remove a todo
    Remove {
        id: String,
    },
    /// Clear completed todos
    Clear,
    /// Edit a todo
    Edit {
        id: String,
        text: Option<String>,
        priority: Option<String>,
        notes: Option<String>,
    },
}

/// Tool for managing todo lists.
pub struct TodoTool;

impl TodoTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for TodoTool {
    fn name(&self) -> &'static str {
        "todo"
    }

    fn description(&self) -> &'static str {
        "Manage a todo list: add, list, complete, remove tasks"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["action".to_string()];
        
        // Define action enum
        schema.properties.insert("action".to_string(), ToolParameter {
            param_type: "string".to_string(),
            description: "Action: add, list, complete, uncomplete, remove, clear, edit".to_string(),
            default: None,
            enum_values: Some(vec![
                serde_json::json!("add"),
                serde_json::json!("list"),
                serde_json::json!("complete"),
                serde_json::json!("uncomplete"),
                serde_json::json!("remove"),
                serde_json::json!("clear"),
                serde_json::json!("edit"),
            ]),
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: None,
        });
        
        schema.properties.insert("text".to_string(), ToolParameter::string("Todo text (for add)"));
        schema.properties.insert("id".to_string(), ToolParameter::string("Todo ID (for complete/remove/edit)"));
        schema.properties.insert("priority".to_string(), ToolParameter::string("Priority: low, medium, high"));
        schema.properties.insert("notes".to_string(), ToolParameter::string("Optional notes"));
        schema.properties.insert("status".to_string(), ToolParameter::string("Filter by status: pending, completed"));
        
        schema
    }

    async fn execute(&self, params: serde_json::Value, _context: &ToolContext) -> ToolResult<ToolOutput> {
        let action: TodoAction = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let list_guard = get_todo_list();
        let mut list = list_guard.lock().await;
        
        let output = match &action {
            TodoAction::Add { text, priority, notes } => {
                let id = ulid::Ulid::new().to_string();
                let item = TodoItem {
                    id: id.clone(),
                    text: text.clone(),
                    completed: false,
                    priority: priority.clone(),
                    notes: notes.clone(),
                    created_at: Some(chrono::Utc::now().to_rfc3339()),
                };
                list.items.push(item);
                format!("✓ Added todo #{}: {}",
                    list.items.len(),
                    list.items.last().unwrap().text)
            }

            TodoAction::List { status, priority } => {
                let mut filtered = list.items.clone();

                if let Some(status) = status {
                    match status.as_str() {
                        "completed" | "done" => {
                            filtered.retain(|i| i.completed);
                        }
                        "pending" | "active" | "incomplete" => {
                            filtered.retain(|i| !i.completed);
                        }
                        _ => {}
                    }
                }

                if let Some(priority) = priority {
                    filtered.retain(|i| i.priority.as_ref().map(|p| p == priority).unwrap_or(false));
                }

                if filtered.is_empty() {
                    "No todos found".to_string()
                } else {
                    let mut output = format!("Todo List ({} item(s)):\n\n", filtered.len());
                    for (i, item) in filtered.iter().enumerate() {
                        let checkbox = if item.completed { "✓" } else { "○" };
                        let priority = item.priority.as_ref()
                            .map(|p| format!(" [{}]", p))
                            .unwrap_or_default();
                        output.push_str(&format!("{}. {} #{} {}{}\n",
                            i + 1, checkbox, item.id[..8].to_string(), item.text, priority));
                    }
                    output
                }
            }

            TodoAction::Complete { id } => {
                if let Some(item) = list.items.iter_mut().find(|i| i.id == *id) {
                    item.completed = true;
                    format!("✓ Completed: {}", item.text)
                } else {
                    return Err(ToolError::Execution(format!("Todo not found: {}", id)));
                }
            }

            TodoAction::Uncomplete { id } => {
                if let Some(item) = list.items.iter_mut().find(|i| i.id == *id) {
                    item.completed = false;
                    format!("○ Reopened: {}", item.text)
                } else {
                    return Err(ToolError::Execution(format!("Todo not found: {}", id)));
                }
            }

            TodoAction::Remove { id } => {
                let original_len = list.items.len();
                list.items.retain(|i| i.id != *id);
                if list.items.len() < original_len {
                    format!("✓ Removed todo #{}", id)
                } else {
                    return Err(ToolError::Execution(format!("Todo not found: {}", id)));
                }
            }

            TodoAction::Clear => {
                let count = list.items.iter().filter(|i| i.completed).count();
                list.items.retain(|i| !i.completed);
                format!("✓ Cleared {} completed todo(s)", count)
            }

            TodoAction::Edit { id, text, priority, notes } => {
                if let Some(item) = list.items.iter_mut().find(|i| i.id == *id) {
                    let mut changes = Vec::new();
                    if let Some(t) = text {
                        item.text = t.clone();
                        changes.push(format!("text: {}", t));
                    }
                    if let Some(p) = priority {
                        item.priority = Some(p.clone());
                        changes.push(format!("priority: {}", p));
                    }
                    if let Some(n) = notes {
                        item.notes = Some(n.clone());
                        changes.push(format!("notes: {}", n));
                    }
                    format!("✓ Updated todo #{}: {}", id, changes.join(", "))
                } else {
                    return Err(ToolError::Execution(format!("Todo not found: {}", id)));
                }
            }
        };

        // Generate summary
        let pending = list.items.iter().filter(|i| !i.completed).count();
        let completed = list.items.iter().filter(|i| i.completed).count();

        Ok(ToolOutput::success(output).with_data(serde_json::json!({
            "action": format!("{:?}", action).to_lowercase(),
            "pending": pending,
            "completed": completed,
            "total": list.items.len(),
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_todo_add() {
        let tool = TodoTool::new();
        let params = serde_json::json!({
            "action": "add",
            "text": "Test task",
            "priority": "high"
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("Added todo"));
    }

    #[tokio::test]
    async fn test_todo_list() {
        let tool = TodoTool::new();
        let params = serde_json::json!({
            "action": "list"
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_todo_complete() {
        // First add a todo
        let add_tool = TodoTool::new();
        let add_params = serde_json::json!({
            "action": "add",
            "text": "Task to complete"
        });
        let context = ToolContext::default();
        let add_result = add_tool.execute(add_params, &context).await.unwrap();
        
        // Extract ID from data
        let _data = add_result.data.unwrap();
        let binding = get_todo_list();
        let list = binding.lock().await;
        if let Some(item) = list.items.last() {
            let id = item.id.clone();
            drop(list);
            
            // Complete it
            let tool = TodoTool::new();
            let params = serde_json::json!({
                "action": "complete",
                "id": id
            });
            
            let result = tool.execute(params, &context).await;
            assert!(result.is_ok());
            let output = result.unwrap();
            assert!(output.output.contains("Completed"));
        }
    }
}
