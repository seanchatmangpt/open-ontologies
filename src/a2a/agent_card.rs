//! T2-2 Agent card generation for A2A discovery.

use serde::{Deserialize, Serialize};

/// Lightweight agent info structure for A2A discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleAgentInfo {
    pub name: String,
    pub url: String,
    pub version: String,
    pub skills: Vec<String>,
}

/// Build an A2A agent info card with core onto_* capabilities.
///
/// Exposes 5 core A2A skills:
/// - onto_status: Server health and ontology status
/// - onto_query: SPARQL query execution
/// - onto_validate: Ontology validation via SHACL
/// - onto_load: Load TTL/RDF into store
/// - onto_stats: Retrieve triple counts and metrics
pub fn build_agent_info(agent_name: &str, agent_url: &str) -> SimpleAgentInfo {
    SimpleAgentInfo {
        name: agent_name.to_string(),
        url: agent_url.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        skills: vec![
            "onto_status".to_string(),
            "onto_query".to_string(),
            "onto_validate".to_string(),
            "onto_load".to_string(),
            "onto_stats".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_info_has_five_skills() {
        let info = build_agent_info("test-agent", "http://localhost:8080");
        assert_eq!(info.skills.len(), 5);
        assert!(info.skills.contains(&"onto_status".to_string()));
    }
}
