//! Plan Tool — enter/exit plan mode

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// Plan mode state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanState {
    /// Whether plan mode is active
    pub active: bool,
    /// Current plan steps
    pub steps: Vec<PlanStep>,
    /// Current step index
    pub current_step: Option<usize>,
    /// Plan metadata
    pub metadata: PlanMetadata,
}

/// A single plan step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Step number
    pub index: usize,
    /// Step description
    pub description: String,
    /// Step status
    pub status: PlanStepStatus,
    /// Optional notes
    pub notes: Option<String>,
}

/// Plan step status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlanStepStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
    Failed,
}

impl Default for PlanStepStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Plan metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanMetadata {
    /// Plan title
    pub title: Option<String>,
    /// Created timestamp
    pub created_at: Option<String>,
    /// Last updated timestamp
    pub updated_at: Option<String>,
}

// Global plan state (in production, use persistent storage)
static PLAN_STATE: OnceLock<Arc<Mutex<PlanState>>> = OnceLock::new();

fn get_plan_state() -> Arc<Mutex<PlanState>> {
    PLAN_STATE
        .get_or_init(|| Arc::new(Mutex::new(PlanState::default())))
        .clone()
}

/// Parameters for the plan tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PlanAction {
    /// Enter plan mode with a plan
    Enter {
        title: Option<String>,
        steps: Vec<String>,
    },
    /// Exit plan mode
    Exit,
    /// Get current plan status
    Status,
    /// Update a step's status
    Update {
        step_index: usize,
        status: String,
        notes: Option<String>,
    },
    /// Add a new step
    AddStep {
        description: String,
        after: Option<usize>,
    },
    /// Remove a step
    RemoveStep {
        step_index: usize,
    },
    /// Mark current step as complete and move to next
    Next,
}

/// Tool for managing plan mode.
pub struct PlanTool;

impl PlanTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for PlanTool {
    fn name(&self) -> &'static str {
        "plan"
    }

    fn description(&self) -> &'static str {
        "Enter or exit plan mode, manage plan steps"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["action".to_string()];
        
        schema.properties.insert("action".to_string(), ToolParameter {
            param_type: "string".to_string(),
            description: "Action: enter, exit, status, update, add_step, remove_step, next".to_string(),
            default: None,
            enum_values: Some(vec![
                serde_json::json!("enter"),
                serde_json::json!("exit"),
                serde_json::json!("status"),
                serde_json::json!("update"),
                serde_json::json!("add_step"),
                serde_json::json!("remove_step"),
                serde_json::json!("next"),
            ]),
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: None,
        });
        
        schema.properties.insert("title".to_string(), ToolParameter::string("Plan title"));
        schema.properties.insert("steps".to_string(), ToolParameter {
            param_type: "array".to_string(),
            description: "List of plan steps".to_string(),
            default: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: Some(Box::new(ToolParameter::string("Step description"))),
        });
        schema.properties.insert("step_index".to_string(), ToolParameter::integer("Step index (0-based)"));
        schema.properties.insert("status".to_string(), ToolParameter::string("Status: pending, in_progress, completed, skipped, failed"));
        schema.properties.insert("description".to_string(), ToolParameter::string("Step description"));
        schema.properties.insert("notes".to_string(), ToolParameter::string("Optional notes"));
        schema.properties.insert("after".to_string(), ToolParameter::integer("Insert after this step index"));
        
        schema
    }

    async fn execute(&self, params: serde_json::Value, _context: &ToolContext) -> ToolResult<ToolOutput> {
        let action: PlanAction = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let state_guard = get_plan_state();
        let mut state = state_guard.lock().await;
        
        let output = match action {
            PlanAction::Enter { title, steps } => {
                if steps.is_empty() {
                    return Err(ToolError::InvalidParams("Plan must have at least one step".to_string()));
                }
                
                state.active = true;
                state.steps = steps.iter().enumerate().map(|(i, s)| PlanStep {
                    index: i,
                    description: s.clone(),
                    status: if i == 0 { PlanStepStatus::InProgress } else { PlanStepStatus::Pending },
                    notes: None,
                }).collect();
                state.current_step = Some(0);
                state.metadata = PlanMetadata {
                    title,
                    created_at: Some(chrono::Utc::now().to_rfc3339()),
                    updated_at: Some(chrono::Utc::now().to_rfc3339()),
                };
                
                format!("✓ Entered plan mode with {} step(s)\n\n{}", 
                    state.steps.len(),
                    format_plan(&state))
            }
            
            PlanAction::Exit => {
                if !state.active {
                    "Plan mode is not active".to_string()
                } else {
                    state.active = false;
                    state.current_step = None;
                    "✓ Exited plan mode".to_string()
                }
            }
            
            PlanAction::Status => {
                if !state.active {
                    "Plan mode is not active".to_string()
                } else {
                    format_plan(&state)
                }
            }
            
            PlanAction::Update { step_index, status, notes } => {
                if !state.active {
                    return Err(ToolError::Execution("Plan mode is not active".to_string()));
                }
                
                if step_index >= state.steps.len() {
                    return Err(ToolError::InvalidParams(format!(
                        "Step index {} out of range (0-{})", 
                        step_index, state.steps.len() - 1
                    )));
                }
                
                let step_status = match status.as_str() {
                    "pending" => PlanStepStatus::Pending,
                    "in_progress" | "inprogress" | "active" => PlanStepStatus::InProgress,
                    "completed" | "done" => PlanStepStatus::Completed,
                    "skipped" => PlanStepStatus::Skipped,
                    "failed" | "error" => PlanStepStatus::Failed,
                    _ => return Err(ToolError::InvalidParams(format!("Unknown status: {}", status))),
                };
                
                state.steps[step_index].status = step_status;
                state.steps[step_index].notes = notes;
                state.metadata.updated_at = Some(chrono::Utc::now().to_rfc3339());
                
                format!("✓ Updated step {}: {}\n\n{}", 
                    step_index,
                    state.steps[step_index].description,
                    format_plan(&state))
            }
            
            PlanAction::AddStep { description, after } => {
                if !state.active {
                    return Err(ToolError::Execution("Plan mode is not active".to_string()));
                }
                
                let insert_index = match after {
                    Some(idx) if idx < state.steps.len() => idx + 1,
                    Some(_) => return Err(ToolError::InvalidParams(format!(
                        "Invalid after index: {}", after.unwrap()
                    ))),
                    None => state.steps.len(),
                };
                
                let new_step = PlanStep {
                    index: insert_index,
                    description,
                    status: PlanStepStatus::Pending,
                    notes: None,
                };
                
                state.steps.insert(insert_index, new_step);
                
                // Re-index all steps
                for (i, step) in state.steps.iter_mut().enumerate() {
                    step.index = i;
                }
                
                state.metadata.updated_at = Some(chrono::Utc::now().to_rfc3339());
                
                format!("✓ Added step at position {}\n\n{}", insert_index, format_plan(&state))
            }
            
            PlanAction::RemoveStep { step_index } => {
                if !state.active {
                    return Err(ToolError::Execution("Plan mode is not active".to_string()));
                }
                
                if step_index >= state.steps.len() {
                    return Err(ToolError::InvalidParams(format!(
                        "Step index {} out of range", step_index
                    )));
                }
                
                let removed = state.steps.remove(step_index);
                
                // Re-index all steps
                for (i, step) in state.steps.iter_mut().enumerate() {
                    step.index = i;
                }
                
                // Update current step if needed
                if let Some(current) = state.current_step {
                    if current >= step_index {
                        state.current_step = Some(current.saturating_sub(1));
                    }
                }
                
                state.metadata.updated_at = Some(chrono::Utc::now().to_rfc3339());
                
                format!("✓ Removed step: {}\n\n{}", removed.description, format_plan(&state))
            }
            
            PlanAction::Next => {
                if !state.active {
                    return Err(ToolError::Execution("Plan mode is not active".to_string()));
                }
                
                if let Some(current) = state.current_step {
                    if current < state.steps.len() {
                        state.steps[current].status = PlanStepStatus::Completed;
                        
                        if current + 1 < state.steps.len() {
                            state.steps[current + 1].status = PlanStepStatus::InProgress;
                            state.current_step = Some(current + 1);
                            
                            format!("✓ Completed step {}\n→ Moving to step {}\n\n{}", 
                                current,
                                current + 1,
                                format_plan(&state))
                        } else {
                            state.current_step = None;
                            "✓ All steps completed!\n\n".to_string() + &format_plan(&state)
                        }
                    } else {
                        "All steps completed".to_string()
                    }
                } else {
                    "No current step".to_string()
                }
            }
        };

        Ok(ToolOutput::success(output).with_data(serde_json::json!({
            "active": state.active,
            "total_steps": state.steps.len(),
            "completed": state.steps.iter().filter(|s| s.status == PlanStepStatus::Completed).count(),
            "current_step": state.current_step,
        })))
    }
}

/// Format plan state for display.
fn format_plan(state: &PlanState) -> String {
    let mut output = String::new();
    
    if let Some(title) = &state.metadata.title {
        output.push_str(&format!("**{}**\n\n", title));
    }
    
    for step in &state.steps {
        let icon = match step.status {
            PlanStepStatus::Pending => "○",
            PlanStepStatus::InProgress => "→",
            PlanStepStatus::Completed => "✓",
            PlanStepStatus::Skipped => "⊘",
            PlanStepStatus::Failed => "✗",
        };
        
        let current_marker = if Some(step.index) == state.current_step { " ← CURRENT" } else { "" };
        
        output.push_str(&format!("{}. {} {}{}\n", 
            step.index + 1, icon, step.description, current_marker));
        
        if let Some(notes) = &step.notes {
            output.push_str(&format!("   Notes: {}\n", notes));
        }
    }
    
    let completed = state.steps.iter().filter(|s| s.status == PlanStepStatus::Completed).count();
    let total = state.steps.len();
    let progress = if total > 0 { (completed as f64 / total as f64 * 100.0) as u32 } else { 0 };
    
    output.push_str(&format!("\nProgress: {}/{} ({}%)", completed, total, progress));
    
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plan_enter() {
        let tool = PlanTool::new();
        let params = serde_json::json!({
            "action": "enter",
            "title": "Test Plan",
            "steps": ["Step 1", "Step 2", "Step 3"]
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("Test Plan"));
        assert!(output.output.contains("Step 1"));
    }

    #[tokio::test]
    async fn test_plan_status() {
        // First enter plan mode
        let enter_tool = PlanTool::new();
        let enter_params = serde_json::json!({
            "action": "enter",
            "steps": ["Do something"]
        });
        let context = ToolContext::default();
        enter_tool.execute(enter_params, &context).await.unwrap();
        
        // Then check status
        let tool = PlanTool::new();
        let params = serde_json::json!({
            "action": "status"
        });
        
        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        assert!(result.unwrap().output.contains("Do something"));
    }

    #[tokio::test]
    async fn test_plan_next() {
        let context = ToolContext::default();
        // Clear any prior state (global PLAN_STATE is shared across tests)
        let exit_tool = PlanTool::new();
        let _ = exit_tool.execute(serde_json::json!({ "action": "exit" }), &context).await;
        // Enter plan mode
        let enter_tool = PlanTool::new();
        let enter_params = serde_json::json!({
            "action": "enter",
            "steps": ["Step 1", "Step 2"]
        });
        enter_tool.execute(enter_params, &context).await.unwrap();
        // Move to next
        let tool = PlanTool::new();
        let params = serde_json::json!({
            "action": "next"
        });
        
        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(
            output.output.contains("Completed step") || output.output.contains("All steps completed"),
            "output: {}",
            output.output
        );
    }
}
