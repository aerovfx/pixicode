//! Agent types — Agent definitions and configurations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Agent type definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// Full access agent (build)
    Build,
    /// Read-only agent (plan)
    Plan,
    /// General purpose sub-agent
    General,
    /// Research specialist
    Researcher,
    /// Code specialist
    Coder,
    /// Review specialist
    Reviewer,
    /// Custom agent
    Custom,
}

/// Agent permission level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentPermission {
    /// No permissions
    None,
    /// Read files
    Read,
    /// Write files
    Write,
    /// Execute commands
    Execute,
    /// Access web
    Web,
    /// Use tools
    Tools,
    /// Full access
    Full,
}

/// Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent type
    #[serde(rename = "type")]
    pub agent_type: AgentType,
    /// Agent name
    pub name: String,
    /// Agent description
    pub description: String,
    /// System prompt for the agent
    pub system_prompt: String,
    /// Permission level
    pub permissions: AgentPermission,
    /// Allowed tools
    pub allowed_tools: Vec<String>,
    /// Model to use
    pub model: String,
    /// Temperature
    pub temperature: Option<f32>,
    /// Max tokens
    pub max_tokens: Option<u32>,
    /// Custom settings
    pub settings: HashMap<String, serde_json::Value>,
}

impl AgentConfig {
    pub fn build() -> Self {
        Self {
            agent_type: AgentType::Build,
            name: "build".to_string(),
            description: "Full access agent with all permissions".to_string(),
            system_prompt: "You are a build agent with full access to all tools and files. Help users build software efficiently.".to_string(),
            permissions: AgentPermission::Full,
            allowed_tools: vec!["*".to_string()],
            model: "gpt-4".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(4096),
            settings: HashMap::new(),
        }
    }

    pub fn plan() -> Self {
        Self {
            agent_type: AgentType::Plan,
            name: "plan".to_string(),
            description: "Read-only agent for planning and analysis".to_string(),
            system_prompt: "You are a planning agent. Analyze problems and create detailed plans. You have read-only access.".to_string(),
            permissions: AgentPermission::Read,
            allowed_tools: vec!["read".to_string(), "ls".to_string(), "glob".to_string(), "grep".to_string()],
            model: "gpt-4".to_string(),
            temperature: Some(0.5),
            max_tokens: Some(4096),
            settings: HashMap::new(),
        }
    }

    pub fn general() -> Self {
        Self {
            agent_type: AgentType::General,
            name: "general".to_string(),
            description: "General purpose sub-agent for complex tasks".to_string(),
            system_prompt: "You are a general purpose assistant. Help users with various tasks including research, coding, and analysis.".to_string(),
            permissions: AgentPermission::Tools,
            allowed_tools: vec!["*".to_string()],
            model: "gpt-3.5-turbo".to_string(),
            temperature: Some(0.7),
            max_tokens: Some(2048),
            settings: HashMap::new(),
        }
    }

    pub fn researcher() -> Self {
        Self {
            agent_type: AgentType::Researcher,
            name: "researcher".to_string(),
            description: "Research specialist for gathering information".to_string(),
            system_prompt: "You are a research specialist. Gather information from various sources and synthesize findings.".to_string(),
            permissions: AgentPermission::Web,
            allowed_tools: vec!["webfetch".to_string(), "websearch".to_string(), "read".to_string(), "glob".to_string()],
            model: "gpt-4".to_string(),
            temperature: Some(0.3),
            max_tokens: Some(4096),
            settings: HashMap::new(),
        }
    }

    pub fn coder() -> Self {
        Self {
            agent_type: AgentType::Coder,
            name: "coder".to_string(),
            description: "Code specialist for implementation".to_string(),
            system_prompt: "You are a coding specialist. Write clean, efficient, and well-tested code.".to_string(),
            permissions: AgentPermission::Write,
            allowed_tools: vec!["read".to_string(), "write".to_string(), "edit".to_string(), "multiedit".to_string(), "bash".to_string()],
            model: "gpt-4".to_string(),
            temperature: Some(0.2),
            max_tokens: Some(4096),
            settings: HashMap::new(),
        }
    }

    pub fn reviewer() -> Self {
        Self {
            agent_type: AgentType::Reviewer,
            name: "reviewer".to_string(),
            description: "Code review specialist".to_string(),
            system_prompt: "You are a code review specialist. Provide thorough, constructive feedback on code quality, security, and best practices.".to_string(),
            permissions: AgentPermission::Read,
            allowed_tools: vec!["read".to_string(), "ls".to_string(), "glob".to_string(), "grep".to_string(), "codesearch".to_string()],
            model: "gpt-4".to_string(),
            temperature: Some(0.3),
            max_tokens: Some(4096),
            settings: HashMap::new(),
        }
    }
}

/// An agent instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Unique agent ID
    pub id: String,
    /// Agent configuration
    pub config: AgentConfig,
    /// Whether agent is active
    pub active: bool,
    /// Custom instructions
    pub custom_instructions: Option<String>,
}

impl Agent {
    pub fn new(id: String, config: AgentConfig) -> Self {
        Self {
            id,
            config,
            active: true,
            custom_instructions: None,
        }
    }

    pub fn from_config(config: AgentConfig) -> Self {
        Self::new(config.name.clone(), config)
    }

    pub fn with_instructions(mut self, instructions: String) -> Self {
        self.custom_instructions = Some(instructions);
        self
    }

    /// Get effective system prompt.
    pub fn system_prompt(&self) -> String {
        let mut prompt = self.config.system_prompt.clone();
        if let Some(ref instructions) = self.custom_instructions {
            prompt.push_str("\n\n");
            prompt.push_str(instructions);
        }
        prompt
    }

    /// Check if tool is allowed.
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        if self.config.allowed_tools.contains(&"*".to_string()) {
            return true;
        }
        self.config.allowed_tools.contains(&tool_name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_agents() {
        let build = AgentConfig::build();
        assert_eq!(build.agent_type, AgentType::Build);
        assert_eq!(build.permissions, AgentPermission::Full);

        let plan = AgentConfig::plan();
        assert_eq!(plan.agent_type, AgentType::Plan);
        assert_eq!(plan.permissions, AgentPermission::Read);
    }

    #[test]
    fn test_agent_tool_permission() {
        let agent = Agent::from_config(AgentConfig::plan());
        assert!(agent.is_tool_allowed("read"));
        assert!(!agent.is_tool_allowed("write"));
    }
}
