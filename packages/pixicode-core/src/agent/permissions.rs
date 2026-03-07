//! Agent Permissions — Permission checking for agents

use crate::agent::types::{Agent, AgentPermission};
use crate::tools::trait_def::Tool;

/// Permission checker for agents.
pub struct PermissionChecker;

impl PermissionChecker {
    /// Check if agent can execute a tool.
    pub fn can_execute_tool(agent: &Agent, tool: &dyn Tool) -> bool {
        match agent.config.permissions {
            AgentPermission::None => false,
            AgentPermission::Read => Self::is_read_tool(tool.name()),
            AgentPermission::Write => Self::is_read_tool(tool.name()) || Self::is_write_tool(tool.name()),
            AgentPermission::Execute => Self::is_read_tool(tool.name()) || Self::is_write_tool(tool.name()) || tool.name() == "bash",
            AgentPermission::Web => Self::is_read_tool(tool.name()) || Self::is_web_tool(tool.name()),
            AgentPermission::Tools => agent.is_tool_allowed(tool.name()),
            AgentPermission::Full => true,
        }
    }

    /// Check if agent can read files.
    pub fn can_read(agent: &Agent) -> bool {
        matches!(
            agent.config.permissions,
            AgentPermission::Read | AgentPermission::Write | AgentPermission::Execute |
            AgentPermission::Web | AgentPermission::Tools | AgentPermission::Full
        )
    }

    /// Check if agent can write files.
    pub fn can_write(agent: &Agent) -> bool {
        matches!(
            agent.config.permissions,
            AgentPermission::Write | AgentPermission::Execute | 
            AgentPermission::Tools | AgentPermission::Full
        )
    }

    /// Check if agent can execute commands.
    pub fn can_execute(agent: &Agent) -> bool {
        matches!(
            agent.config.permissions,
            AgentPermission::Execute | AgentPermission::Tools | AgentPermission::Full
        )
    }

    /// Check if agent can access web.
    pub fn can_access_web(agent: &Agent) -> bool {
        matches!(
            agent.config.permissions,
            AgentPermission::Web | AgentPermission::Tools | AgentPermission::Full
        )
    }

    fn is_read_tool(name: &str) -> bool {
        matches!(name, "read" | "ls" | "glob" | "grep" | "codesearch")
    }

    fn is_write_tool(name: &str) -> bool {
        matches!(name, "write" | "edit" | "multiedit" | "apply_patch")
    }

    fn is_web_tool(name: &str) -> bool {
        matches!(name, "webfetch" | "websearch")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::types::AgentConfig;

    #[test]
    fn test_permission_checker() {
        let plan_agent = Agent::from_config(AgentConfig::plan());
        let build_agent = Agent::from_config(AgentConfig::build());

        assert!(PermissionChecker::can_read(&plan_agent));
        assert!(!PermissionChecker::can_write(&plan_agent));
        
        assert!(PermissionChecker::can_read(&build_agent));
        assert!(PermissionChecker::can_write(&build_agent));
        assert!(PermissionChecker::can_execute(&build_agent));
    }
}
