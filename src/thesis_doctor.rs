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

    // Check 3: Gemini connectivity check
    let gemini_bin = crate::config::resolve_gemini_bin();
    let mut cmd = std::process::Command::new(&gemini_bin);
    if gemini_bin == "npx" {
        cmd.arg("-y").arg("@google/gemini-cli");
    }
    cmd.arg("--version");

    let (gemini_ok, gemini_msg) = match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                let stdout_str = String::from_utf8_lossy(&output.stdout);
                let version = stdout_str.trim();
                (
                    true,
                    format!("Gemini CLI reachable: version {}", version),
                )
            } else {
                let stderr_str = String::from_utf8_lossy(&output.stderr);
                let stdout_str = String::from_utf8_lossy(&output.stdout);
                let err_msg = if !stderr_str.trim().is_empty() {
                    stderr_str.trim().to_string()
                } else if !stdout_str.trim().is_empty() {
                    stdout_str.trim().to_string()
                } else {
                    format!("exit status {}", output.status.code().unwrap_or(-1))
                };
                (
                    false,
                    format!("Gemini CLI failed: {}", err_msg),
                )
            }
        }
        Err(e) => {
            (
                false,
                format!("Gemini CLI command failed to start: {}", e),
            )
        }
    };

    checks.push((
        "Gemini Connectivity".to_string(),
        gemini_ok,
        gemini_msg,
    ));

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
        assert_eq!(checks[2].0, "Gemini Connectivity");
    }
}

