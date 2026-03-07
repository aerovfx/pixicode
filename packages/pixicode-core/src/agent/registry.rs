//! Agent Registry — Agent registration and lookup

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agent::types::{Agent, AgentConfig, AgentType};
use crate::config::types::parse_jsonc;

/// Agent registry.
pub struct AgentRegistry {
    agents: RwLock<HashMap<String, Arc<Agent>>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_defaults() -> Self {
        let registry = Self::new();
        registry.register_defaults();
        registry
    }

    pub async fn register(&self, agent: Agent) {
        let id = agent.id.clone();
        self.agents.write().await.insert(id, Arc::new(agent));
    }

    pub async fn get(&self, id: &str) -> Option<Arc<Agent>> {
        self.agents.read().await.get(id).cloned()
    }

    pub async fn list(&self) -> Vec<Arc<Agent>> {
        self.agents.read().await.values().cloned().collect()
    }

    pub async fn list_by_type(&self, agent_type: AgentType) -> Vec<Arc<Agent>> {
        self.agents.read().await.values()
            .filter(|a| a.config.agent_type == agent_type)
            .cloned()
            .collect()
    }

    fn register_defaults(&self) {
        futures::executor::block_on(async {
            self.register(Agent::from_config(AgentConfig::build())).await;
            self.register(Agent::from_config(AgentConfig::plan())).await;
            self.register(Agent::from_config(AgentConfig::general())).await;
            self.register(Agent::from_config(AgentConfig::researcher())).await;
            self.register(Agent::from_config(AgentConfig::coder())).await;
            self.register(Agent::from_config(AgentConfig::reviewer())).await;
        });
    }

    /// Load and register agent configs from `.pixicode/agent/` (JSON/JSONC files).
    /// Returns the number of agents loaded. Skips invalid files.
    pub async fn load_from_dir(&self, project_root: &Path) -> usize {
        let agent_dir = project_root.join(".pixicode").join("agent");
        if !agent_dir.is_dir() {
            return 0;
        }
        let mut count = 0;
        let Ok(entries) = std::fs::read_dir(&agent_dir) else {
            return 0;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str());
            if ext != Some("json") && ext != Some("jsonc") {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(config) = parse_jsonc::<AgentConfig>(&content) else {
                continue;
            };
            self.register(Agent::from_config(config)).await;
            count += 1;
        }
        count
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
