//! R1 smoke test: every builtin POWL string must parse under wasm4pm grammar.

use open_ontologies::powl_bridge::PowlBridge;
use open_ontologies::workflows::builtin::BUILTIN_WORKFLOWS;

#[test]
fn every_builtin_powl_string_parses_under_wasm4pm() {
    let mut failures = Vec::new();
    for wf in BUILTIN_WORKFLOWS {
        let mut bridge = PowlBridge::new();
        match bridge.parse(wf.powl_string) {
            Ok(_) => {}
            Err(e) => failures.push(format!("{}: {}", wf.name, e)),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} builtin workflows failed to parse:\n{}",
        failures.len(),
        BUILTIN_WORKFLOWS.len(),
        failures.join("\n")
    );
}

#[test]
fn catalog_has_ten_entries() {
    // 7 originals + RequirementsManufacturing (Phase 1.2) +
    // Fortune5RevOpsGovernedRelease (Phase 1.2) + SolutionManufacturing (Phase 4).
    assert_eq!(BUILTIN_WORKFLOWS.len(), 10);
}
