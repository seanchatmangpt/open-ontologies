//! Domain logic for thesis doctor health checks.

/// Run all doctor checks for thesis health.
pub fn run_doctor_checks(graph: &crate::graph::GraphStore) -> Vec<(String, bool, String)> {
    let mut checks = vec![];

    // Check 1: RDF store connectivity
    let store_ok = graph
        .sparql_select("SELECT (COUNT(*) AS ?count) WHERE { ?s ?p ?o }")
        .is_ok();
    checks.push((
        "RDF Store".to_string(),
        store_ok,
        if store_ok {
            "RDF store accessible".to_string()
        } else {
            "RDF store unreachable".to_string()
        },
    ));

    // Check 2: Thesis Shapes validation check
    let shapes_path = std::path::Path::new("ontology/thesis-shapes.ttl");
    let shapes_ok = shapes_path.is_file();
    checks.push((
        "Thesis Shapes".to_string(),
        shapes_ok,
        if shapes_ok {
            "ontology/thesis-shapes.ttl located".to_string()
        } else {
            "ontology/thesis-shapes.ttl not found".to_string()
        },
    ));

    // Check 3: Groq connectivity check
    let cfg = crate::config::LlmConfig::default();
    let api_key = crate::config::resolve_llm_api_key(&cfg);
    let api_base = crate::config::resolve_llm_api_base(&cfg);

    let (groq_ok, groq_msg) = if let Some(key) = api_key {
        if !key.trim().is_empty() {
            (
                true,
                format!("Groq API key is present. Base URL: {}", api_base),
            )
        } else {
            (false, "Groq API key is empty".to_string())
        }
    } else {
        (
            false,
            "Groq API key (GROQ_API_KEY or OPEN_ONTOLOGIES_LLM_API_KEY) not found in environment"
                .to_string(),
        )
    };

    checks.push(("Groq Connectivity".to_string(), groq_ok, groq_msg));

    checks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::GraphStore;

    #[test]
    fn test_doctor_checks_returns_correct_number_of_checks() {
        let graph = GraphStore::new();
        let checks = run_doctor_checks(&graph);
        assert_eq!(checks.len(), 3);
        assert_eq!(checks[0].0, "RDF Store");
        assert_eq!(checks[1].0, "Thesis Shapes");
        assert_eq!(checks[2].0, "Groq Connectivity");
    }
}
