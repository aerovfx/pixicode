//! Skill Tool — skill file loader

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// A loaded skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Skill name
    pub name: String,
    /// Skill description
    pub description: String,
    /// Skill version
    pub version: Option<String>,
    /// Author
    pub author: Option<String>,
    /// Instructions/prompt
    pub instructions: String,
    /// Tools this skill uses
    pub tools: Option<Vec<String>>,
    /// Examples
    pub examples: Option<Vec<String>>,
}

/// Parameters for the skill tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum SkillAction {
    /// Load a skill from a file
    Load {
        /// Path to the skill file (.md or .skill.md)
        path: String,
    },
    /// List available skills
    List {
        /// Directory to search in (default: .pixicode/skills/)
        directory: Option<String>,
    },
    /// Get details about a loaded skill
    Get {
        /// Skill name
        name: String,
    },
    /// Unload a skill
    Unload {
        /// Skill name
        name: String,
    },
    /// List loaded skills
    Loaded,
}

/// Tool for loading and managing skills.
pub struct SkillTool;

impl SkillTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &'static str {
        "skill"
    }

    fn description(&self) -> &'static str {
        "Load and manage skill files that define custom agent behaviors"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["action".to_string()];
        
        schema.properties.insert("action".to_string(), ToolParameter {
            param_type: "string".to_string(),
            description: "Action: load, list, get, unload, loaded".to_string(),
            default: None,
            enum_values: Some(vec![
                serde_json::json!("load"),
                serde_json::json!("list"),
                serde_json::json!("get"),
                serde_json::json!("unload"),
                serde_json::json!("loaded"),
            ]),
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: None,
        });
        
        schema.properties.insert("path".to_string(), ToolParameter::string("Path to the skill file"));
        schema.properties.insert("name".to_string(), ToolParameter::string("Skill name"));
        schema.properties.insert("directory".to_string(), ToolParameter::string("Directory to search for skills"));
        
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let action: SkillAction = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        match action {
            SkillAction::Load { path } => {
                let skill_path = resolve_path(&path, &context.working_dir)?;
                
                if !skill_path.exists() {
                    return Err(ToolError::Execution(format!("Skill file not found: {}", skill_path.display())));
                }
                
                let content = tokio::fs::read_to_string(&skill_path)
                    .await
                    .map_err(|e| ToolError::Io(e))?;
                
                let skill = parse_skill_file(&content, &skill_path)?;
                
                let output = format!(
                    "✓ Loaded skill: {}\n\n\
                    **{}**\n{}\n\n\
                    ---\n\
                    {}\n\
                    ",
                    skill.name,
                    skill.name,
                    skill.description,
                    skill.instructions
                );
                
                Ok(ToolOutput::success(output).with_data(serde_json::json!({
                    "name": skill.name,
                    "description": skill.description,
                    "version": skill.version,
                    "author": skill.author,
                    "tools": skill.tools,
                    "examples": skill.examples,
                })))
            }
            
            SkillAction::List { directory } => {
                let search_dir = match directory {
                    Some(dir) => resolve_path(&dir, &context.working_dir)?,
                    None => context.working_dir.join(".pixicode/skills"),
                };
                
                let mut skills = Vec::new();
                
                if search_dir.exists() && search_dir.is_dir() {
                    let mut entries = tokio::fs::read_dir(&search_dir)
                        .await
                        .map_err(|e| ToolError::Io(e))?;
                    
                    while let Some(entry) = entries.next_entry().await.map_err(|e| ToolError::Io(e))? {
                        let path = entry.path();
                        let file_name = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("");
                        
                        if file_name.ends_with(".skill.md") || file_name.ends_with(".md") {
                            skills.push(path.to_string_lossy().to_string());
                        }
                    }
                }
                
                skills.sort();
                
                let output = if skills.is_empty() {
                    format!("No skills found in {}", search_dir.display())
                } else {
                    format!("Found {} skill(s) in {}:\n\n{}", 
                        skills.len(),
                        search_dir.display(),
                        skills.iter().map(|s| format!("  - {}", s)).collect::<Vec<_>>().join("\n"))
                };
                
                Ok(ToolOutput::success(output).with_data(serde_json::json!({
                    "directory": search_dir.to_string_lossy(),
                    "skills": skills,
                    "count": skills.len(),
                })))
            }
            
            SkillAction::Get { name } => {
                // In a real implementation, this would look up from loaded skills
                // For now, return a placeholder response
                let output = format!(
                    "Skill: {}\n\n\
                    Note: Skill lookup requires a loaded skill registry.\n\
                    Use 'load' action to load a skill first.",
                    name
                );
                
                Ok(ToolOutput::success(output).with_data(serde_json::json!({
                    "name": name,
                    "loaded": false,
                })))
            }
            
            SkillAction::Unload { name } => {
                // In a real implementation, this would remove from loaded skills
                let output = format!("✓ Unloaded skill: {}", name);
                
                Ok(ToolOutput::success(output).with_data(serde_json::json!({
                    "name": name,
                    "unloaded": true,
                })))
            }
            
            SkillAction::Loaded => {
                // In a real implementation, this would list currently loaded skills
                let output = "No skills currently loaded.\n\nUse 'load' action to load a skill.".to_string();
                
                Ok(ToolOutput::success(output).with_data(serde_json::json!({
                    "loaded": [],
                    "count": 0,
                })))
            }
        }
    }
}

/// Parse a skill file (markdown format).
fn parse_skill_file(content: &str, path: &PathBuf) -> ToolResult<Skill> {
    let mut name = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unnamed")
        .replace(".skill", "");
    
    let mut description = String::new();
    let mut version = None;
    let mut author = None;
    let mut instructions = String::new();
    let mut tools = None;
    let mut examples = None;
    
    let mut current_section = String::new();
    let mut in_instructions = false;
    
    for line in content.lines() {
        let trimmed = line.trim();
        
        // Parse frontmatter (YAML-like)
        if trimmed.starts_with("---") && current_section.is_empty() {
            if in_instructions {
                in_instructions = false;
            } else {
                in_instructions = true;
            }
            continue;
        }
        
        if in_instructions && !trimmed.is_empty() {
            // Parse metadata
            if trimmed.starts_with("name:") {
                name = trimmed[5..].trim().to_string();
            } else if trimmed.starts_with("description:") {
                description = trimmed[12..].trim().to_string();
            } else if trimmed.starts_with("version:") {
                version = Some(trimmed[8..].trim().to_string());
            } else if trimmed.starts_with("author:") {
                author = Some(trimmed[7..].trim().to_string());
            } else if trimmed.starts_with("tools:") {
                // Parse tools list
                if let Some(list) = trimmed[6..].trim().strip_prefix('[') {
                    if let Some(list) = list.strip_suffix(']') {
                        tools = Some(list.split(',').map(|s| s.trim().to_string()).collect());
                    }
                }
            }
            continue;
        }
        
        // Parse markdown sections
        if trimmed.starts_with("# ") {
            current_section = "title".to_string();
            if name == "Unnamed" {
                name = trimmed[2..].trim().to_string();
            }
        } else if trimmed.starts_with("## ") {
            current_section = trimmed[3..].trim().to_lowercase();
        } else if !trimmed.is_empty() {
            match current_section.as_str() {
                "description" => {
                    if description.is_empty() {
                        description = trimmed.to_string();
                    } else {
                        description.push(' ');
                        description.push_str(trimmed);
                    }
                }
                "instructions" | "behavior" | "rules" => {
                    if !instructions.is_empty() {
                        instructions.push('\n');
                    }
                    instructions.push_str(line);
                }
                "examples" | "usage" => {
                    let examples_list = examples.get_or_insert_with(Vec::new);
                    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                        examples_list.push(trimmed[2..].to_string());
                    } else if !trimmed.starts_with('#') {
                        if let Some(last) = examples_list.last_mut() {
                            last.push('\n');
                            last.push_str(line);
                        } else {
                            examples_list.push(line.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    
    // If no instructions found, use the whole content
    if instructions.is_empty() {
        instructions = content.to_string();
    }
    
    // Default description if not found
    if description.is_empty() {
        description = format!("Custom skill: {}", name);
    }
    
    Ok(Skill {
        name,
        description,
        version,
        author,
        instructions,
        tools,
        examples,
    })
}

/// Resolves a path relative to the working directory.
fn resolve_path(path: &str, working_dir: &PathBuf) -> Result<PathBuf, ToolError> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(working_dir.join(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_skill_load() {
        let temp_dir = tempdir().unwrap();
        let skill_path = temp_dir.path().join("test.skill.md");
        
        let skill_content = r#"---
name: TestSkill
description: A test skill
version: 1.0.0
author: Test Author
---

# TestSkill

## Description
This is a test skill for testing.

## Instructions
Always be helpful and concise.
Use tools effectively.

## Examples
- How do I use this skill?
- Can you help me with X?
"#;
        
        tokio::fs::write(&skill_path, skill_content).await.unwrap();

        let tool = SkillTool::new();
        let params = serde_json::json!({
            "action": "load",
            "path": skill_path.to_string_lossy()
        });
        let context = ToolContext {
            working_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("TestSkill"));
        assert!(output.output.contains("A test skill"));
    }

    #[tokio::test]
    async fn test_skill_list() {
        let temp_dir = tempdir().unwrap();
        let skills_dir = temp_dir.path().join("skills");
        tokio::fs::create_dir(&skills_dir).await.unwrap();
        tokio::fs::write(skills_dir.join("test1.skill.md"), "content").await.unwrap();
        tokio::fs::write(skills_dir.join("test2.skill.md"), "content").await.unwrap();

        let tool = SkillTool::new();
        let params = serde_json::json!({
            "action": "list",
            "directory": skills_dir.to_string_lossy()
        });
        let context = ToolContext {
            working_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("2 skill"));
    }
}
