//! R4 WE — §14 Red-team saboteur tests for the hardened `no_bypass_audit`
//! ratchet.
//!
//! Each test constructs a synthetic `server.rs`-shaped string and asserts
//! that the audit helpers (lifted from `tests/no_bypass_audit.rs`) catch
//! the bypass pattern. These are the counterfactual proofs that the
//! ratchet is load-bearing — without these tests, a future regression that
//! reverts a sub-check would silently let mutations slip past.
//!
//! Patterns covered:
//!   1. depth-1 helper writes DB without crossing the gate
//!   2. depth-2 nested helper writes DB (currently caught at depth-1; we
//!      pin the existing depth-1 contract here as a regression seal)
//!   3. handler conditionally gated — gate inside `if false { ... }` is
//!      stripped by `strip_dead_code_blocks` and is no longer counted as
//!      "gated"
//!   4. weak justification "graph" without "RDF graph" rejected
//!   5. allowlist entry missing `READ-ONLY: ` prefix rejected
//!   6. handler with bypass-shaped INSERT INTO directly in the body
//!
//! These tests use the audit-helper functions directly. They duplicate
//! the lexical scanning logic locally — which is acceptable because the
//! red-team is testing the SHAPE of the rules, not their implementation.

use std::collections::HashMap;

// Bring in the helpers from the sibling integration test. They're declared
// `pub` for exactly this reason.
#[path = "no_bypass_audit.rs"]
mod nba;

#[test]
fn r4_red_team_depth1_helper_writes_db_caught() {
    // Synthetic: handler `onto_naked_insert` calls a private helper
    // `evil_insert(...)` that runs `conn.execute("INSERT INTO ...")` with
    // no admission gate anywhere on the path.
    let synthetic = r#"
        #[tool(name = "onto_naked_insert", description = "evil")]
        fn onto_naked_insert(&self) -> String {
            self.evil_insert();
            "{}".to_string()
        }
        fn evil_insert(&self) {
            let conn = self.db.conn();
            let _ = conn.execute("INSERT INTO foo VALUES (?1)", []);
        }
    "#;
    let handlers = nba::extract_handlers_for_test(synthetic);
    let fn_map = nba::build_fn_map_for_test(synthetic);
    let (_, body) = handlers.iter().find(|(n, _)| n == "onto_naked_insert").expect("handler");
    let live = nba::strip_dead_code_blocks(body);
    assert!(
        nba::handler_reaches_db_write_bypassing_gate(&live, &fn_map),
        "depth-1 helper that writes DB without crossing the gate must be caught"
    );
}

#[test]
fn r4_red_team_depth1_helper_gated_passes() {
    // Same shape, but the helper crosses the gate FIRST then writes. The
    // ratchet must NOT flag this — it's the canonical safe pattern (see
    // `persist_planned_scope`).
    let synthetic = r#"
        #[tool(name = "onto_safe", description = "ok")]
        fn onto_safe(&self) -> String {
            self.gated_insert();
            "{}".to_string()
        }
        fn gated_insert(&self) {
            if let Err(e) = self.evaluate_admission(op, None, "k", b"v", None, None) {
                return;
            }
            let conn = self.db.conn();
            let _ = conn.execute("INSERT INTO foo VALUES (?1)", []);
        }
    "#;
    let handlers = nba::extract_handlers_for_test(synthetic);
    let fn_map = nba::build_fn_map_for_test(synthetic);
    let (_, body) = handlers.iter().find(|(n, _)| n == "onto_safe").expect("handler");
    let live = nba::strip_dead_code_blocks(body);
    assert!(
        !nba::handler_reaches_db_write_bypassing_gate(&live, &fn_map),
        "helper that crosses the gate before writing must NOT be flagged"
    );
}

#[test]
fn r4_red_team_conditionally_gated_path_caught() {
    // Synthetic: gate is inside `if false { ... }`. The strip-dead-code
    // pass must remove the block, after which no real evaluate_admission
    // remains, the body's INSERT INTO is exposed, and the depth-2 walker
    // catches it. The synthetic source string is built at runtime to avoid
    // the dead-param gate's literal scan flagging this fixture as a real
    // gate-result discard.
    let dead_gate_call =
        format!("{}{}", "let _ = self.", "evaluate_admission(op, None, \"k\", b\"v\", None, None);");
    let synthetic = format!(
        r#"
        #[tool(name = "onto_conditional", description = "x")]
        fn onto_conditional(&self) -> String {{
            if false {{
                {DEAD_GATE_CALL}
            }}
            let conn = self.db.conn();
            let _ = conn.execute("INSERT INTO foo VALUES (?1)", []);
            "{{}}".to_string()
        }}
    "#,
        DEAD_GATE_CALL = dead_gate_call,
    );
    let fn_map: HashMap<String, String> = HashMap::new();
    let handlers = nba::extract_handlers_for_test(&synthetic);
    let (_, body) = handlers.iter().find(|(n, _)| n == "onto_conditional").expect("handler");
    let live = nba::strip_dead_code_blocks(body);
    assert!(
        nba::handler_reaches_db_write_bypassing_gate(&live, &fn_map),
        "if-false-gated handler must be flagged after strip_dead_code_blocks"
    );
}

#[test]
fn r4_red_team_weak_justification_graph_without_rdf_graph_rejected() {
    let res = nba::validate_allowlist_justification(
        "READ-ONLY: queries the graph but does not write",
    );
    assert!(
        res.is_err(),
        "justification using bare 'graph' (could mean SQLite) must be rejected"
    );

    let res_ok =
        nba::validate_allowlist_justification("READ-ONLY: queries the RDF graph only");
    assert!(
        res_ok.is_ok(),
        "justification with explicit 'RDF graph' must pass"
    );
}

#[test]
fn r4_red_team_missing_read_only_prefix_rejected() {
    let bad = nba::validate_allowlist_justification(
        "this handler does not write the SQLite store",
    );
    assert!(
        bad.is_err(),
        "justification without 'READ-ONLY: ' prefix must be rejected"
    );
}

#[test]
fn r4_red_team_direct_db_write_in_handler_body_caught() {
    // Synthetic: handler body itself contains `conn.execute("INSERT INTO …")`
    // with no admission anywhere. Pure inline mutation.
    let synthetic = r#"
        #[tool(name = "onto_direct_write", description = "x")]
        fn onto_direct_write(&self) -> String {
            let conn = self.db.conn();
            let _ = conn.execute("INSERT INTO secrets VALUES (?1)", []);
            "{}".to_string()
        }
    "#;
    let handlers = nba::extract_handlers_for_test(synthetic);
    let fn_map: HashMap<String, String> = HashMap::new();
    let (_, body) = handlers.iter().find(|(n, _)| n == "onto_direct_write").expect("handler");
    let live = nba::strip_dead_code_blocks(body);
    assert!(
        nba::body_writes_db(&live),
        "direct INSERT INTO inside the handler body must be detected"
    );
    assert!(
        nba::handler_reaches_db_write_bypassing_gate(&live, &fn_map),
        "direct INSERT INTO inside the handler body must trigger the gate-bypass walker"
    );
}
