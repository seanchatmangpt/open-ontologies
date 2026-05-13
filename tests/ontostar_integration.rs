//! R10-2 OntoStar integration seal — tests for `onto_ontostar_attest`.
//!
//! Two tests:
//!   1. `onto_ontostar_attest_registered` — tool appears in list_tool_definitions.
//!   2. `onto_ontostar_attest_rejects_bad_signature` — bogus base64 signature
//!      returns JSON with `ok: false`; the server does not panic.

use open_ontologies::inputs::OntoOntostarAttestInput;
use open_ontologies::state::StateDb;
use rmcp::handler::server::wrapper::Parameters;
use tempfile::tempdir;

fn fresh_server() -> open_ontologies::server::OpenOntologiesServer {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("ontostar-integration.db");
    std::mem::forget(dir);
    let db = StateDb::open(&db_path).expect("open StateDb");
    open_ontologies::server::OpenOntologiesServer::new(db)
}

#[test]
fn onto_ontostar_attest_registered() {
    let server = fresh_server();
    let tools = server.list_tool_definitions();
    assert!(
        tools.iter().any(|t| t.name == "onto_ontostar_attest"),
        "onto_ontostar_attest must be registered in tool_router!; got: {:?}",
        tools.iter().map(|t| &t.name).collect::<Vec<_>>()
    );
}

#[test]
fn onto_ontostar_attest_rejects_bad_signature() {
    let server = fresh_server();
    let resp = server.onto_ontostar_attest(Parameters(OntoOntostarAttestInput {
        signature: "not-valid-base64!!!".to_string(),
        payload_hash: "deadbeef".to_string(),
        key_fpr: "0000000000000000".to_string(),
    }));
    let v: serde_json::Value = serde_json::from_str(&resp).expect("response must be valid JSON");
    assert_eq!(
        v["ok"],
        serde_json::Value::Bool(false),
        "bogus signature must yield ok=false; got: {resp}"
    );
    assert!(
        v["error"].is_string(),
        "response must include an error string; got: {resp}"
    );
}
