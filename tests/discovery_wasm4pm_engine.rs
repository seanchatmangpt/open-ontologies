//! Loop-3 workflow discovery bound to the public `wasm4pm` engine (v26.9.26).
//!
//! `wasm4pm-algos` no longer exists at any public revision, so
//! `src/feedback/discovery.rs` now runs Alpha++ discovery and token replay
//! through `wasm4pm::algorithms` / `wasm4pm::conformance`. Every test here uses
//! the real engine and a real SQLite `StateDb`; assertions are on returned
//! values and persisted rows (Chicago style, no test doubles).

use std::collections::HashMap;

use open_ontologies::feedback::discovery::{
    ACTIVITY_KEY, ADMITTED_SCOPES_THRESHOLD, FITNESS_LIFT, build_event_log, discover_for_domain,
    mine_fitness, record_feedback,
};
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use wasm4pm::models::AttributeValue;

fn traces(cases: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
    cases
        .iter()
        .map(|(c, acts)| (c.to_string(), acts.iter().map(|a| a.to_string()).collect()))
        .collect()
}

fn activity(ev: &wasm4pm::models::Event) -> String {
    match ev.attributes.get(ACTIVITY_KEY) {
        Some(AttributeValue::String(s)) => s.clone(),
        other => panic!("event without string activity: {other:?}"),
    }
}

#[test]
fn sequential_log_is_mined_and_every_case_conforms() {
    let log = build_event_log(&traces(&[
        ("c1", &["register", "check", "approve"]),
        ("c2", &["register", "check", "approve"]),
        ("c3", &["register", "check", "approve"]),
    ]));
    let (net, fitness) = mine_fitness(&log).expect("alpha++ + replay on a sequential log");
    let labels: Vec<String> = net.transitions.iter().map(|t| t.label.clone()).collect();
    for act in ["register", "check", "approve"] {
        assert!(
            labels.iter().any(|l| l == act),
            "missing transition {act}: {labels:?}"
        );
    }
    assert!(!net.places.is_empty(), "a replayable net needs places");
    assert!(
        !net.final_markings.is_empty(),
        "discovered net must declare a final marking"
    );

    // The fitness mine_fitness reports is exactly the engine's avg_fitness.
    let conf = wasm4pm::conformance::token_replay_pure(&log, &net, ACTIVITY_KEY);
    assert_eq!(fitness, conf.avg_fitness);
    assert_eq!(conf.total_cases, 3);
    assert_eq!(
        conf.conforming_cases, 3,
        "every case of its own log must conform"
    );
    for case in &conf.case_fitness {
        assert!(case.is_conforming, "{case:?}");
        assert_eq!(case.tokens_missing, 0, "{case:?}");
    }
    // Observed wasm4pm@057d5ba2 accounting: the sink token is counted as
    // remaining, so a conforming 3-step trace scores 5/6, not 1.0. Discovery
    // compares fitness relatively (discovered > declared + FITNESS_LIFT), so
    // the engine's number is used as-is rather than re-derived here.
    assert!(
        fitness > 0.5 && fitness <= 1.0,
        "fitness out of range: {fitness}"
    );
}

#[test]
fn deviating_trace_scores_strictly_lower_than_conforming_ones() {
    let model_log = build_event_log(&traces(&[
        ("c1", &["a", "b", "c"]),
        ("c2", &["a", "b", "c"]),
    ]));
    let (net, _) = mine_fitness(&model_log).unwrap();
    let probe = build_event_log(&traces(&[
        ("good", &["a", "b", "c"]),
        ("skip", &["a", "c"]),
    ]));
    let conf = wasm4pm::conformance::token_replay_pure(&probe, &net, ACTIVITY_KEY);
    assert_eq!(conf.total_cases, 2);
    assert_eq!(
        conf.conforming_cases, 1,
        "a skipped step must not conform: {conf:?}"
    );
    let good = conf.case_fitness.iter().find(|c| c.is_conforming).unwrap();
    let bad = conf.case_fitness.iter().find(|c| !c.is_conforming).unwrap();
    assert!(
        bad.trace_fitness < good.trace_fitness,
        "{bad:?} vs {good:?}"
    );
}

#[test]
fn exclusive_choice_log_is_mined_with_every_activity() {
    let log = build_event_log(&traces(&[
        ("c1", &["a", "b", "d"]),
        ("c2", &["a", "c", "d"]),
        ("c3", &["a", "b", "d"]),
    ]));
    let (net, fitness) = mine_fitness(&log).expect("alpha++ on an XOR log");
    let labels: Vec<String> = net.transitions.iter().map(|t| t.label.clone()).collect();
    for act in ["a", "b", "c", "d"] {
        assert!(
            labels.iter().any(|l| l == act),
            "missing transition {act}: {labels:?}"
        );
    }
    assert!(
        (0.0..=1.0).contains(&fitness),
        "fitness out of range: {fitness}"
    );
}

#[test]
fn single_event_traces_are_a_valid_boundary() {
    let log = build_event_log(&traces(&[("c1", &["only"]), ("c2", &["only"])]));
    let (_net, fitness) = mine_fitness(&log).expect("single-activity log");
    assert!(
        (0.0..=1.0).contains(&fitness),
        "fitness out of range: {fitness}"
    );
}

#[test]
fn empty_log_is_refused_not_scored() {
    let log = build_event_log(&HashMap::new());
    assert!(log.traces.is_empty());
    let err = mine_fitness(&log).expect_err("empty log must be refused");
    assert!(
        err.to_string().contains("REFUSED(EMPTY_LOG)"),
        "unexpected error: {err}"
    );
}

#[test]
fn event_log_projection_is_deterministic_and_ordered_by_case() {
    let input = traces(&[
        ("zeta", &["x", "y"]),
        ("alpha", &["p"]),
        ("mid", &["q", "r", "s"]),
    ]);
    let a = build_event_log(&input);
    let b = build_event_log(&input);
    let case_ids = |log: &wasm4pm::models::EventLog| -> Vec<String> {
        log.traces
            .iter()
            .map(|t| match t.attributes.get(ACTIVITY_KEY) {
                Some(AttributeValue::String(s)) => s.clone(),
                other => panic!("trace without case id: {other:?}"),
            })
            .collect()
    };
    assert_eq!(case_ids(&a), vec!["alpha", "mid", "zeta"]);
    assert_eq!(case_ids(&a), case_ids(&b));
    let mid: Vec<String> = a.traces[1].events.iter().map(activity).collect();
    assert_eq!(
        mid,
        vec!["q", "r", "s"],
        "event order within a case must be preserved"
    );
}

// ── end-to-end over a real StateDb ───────────────────────────────────────────

fn seed_domain(store: &OcelStore, domain: &str, scopes: usize) {
    {
        let conn = store.db().conn();
        for i in 0..scopes {
            let session = format!("sess-{domain}-{i}");
            let scope = format!("scope-{domain}-{i}");
            let receipt = format!("rh-{domain}-{i}");
            conn.execute(
                "INSERT INTO receipts (receipt_hash, scope_token, artifact_hash, declared_powl_hash,
                     ocel_canonical_hash, gate_config_hash, prior_receipt_hash,
                     production_law_version, granted_at, session_id, sequence)
                 VALUES (?1, ?2, 'ah', 'ph', 'oh', 'gh', NULL, 'v1', '2026-09-26T00:00:00Z', ?3, 1)",
                rusqlite::params![receipt, scope, session],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO mined_exemplars (id, domain, problem_context, powl_string, fitness,
                     source_session, receipt_hash, mined_at)
                 VALUES (?1, ?2, 'ctx', 'SEQ(a,b,c)', 1.0, ?3, ?4, '2026-09-26T00:00:00Z')",
                rusqlite::params![format!("ex-{domain}-{i}"), domain, session, receipt],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO declared_workflows (scope_token, session_id, name, powl_string,
                     powl_hash, alphabet_json, declared_at, status)
                 VALUES (?1, ?2, ?3, 'SEQ(a,b,c)', 'ph', '[]', '2026-09-26T00:00:00Z', 'closed')",
                rusqlite::params![scope, session, domain],
            )
            .unwrap();
        }
    }
    for i in 0..scopes {
        let session = format!("sess-{domain}-{i}");
        let scope = format!("scope-{domain}-{i}");
        for (k, act) in ["intake", "review", "ship"].iter().enumerate() {
            store
                .emit_event(
                    &format!("ev-{domain}-{i}-{k}"),
                    act,
                    &format!("2026-09-26T00:{i:02}:{k:02}Z"),
                    &session,
                    &[("scope_token", scope.as_str())],
                    &[],
                    Some(scope.as_str()),
                )
                .unwrap();
        }
    }
}

fn open_store() -> OcelStore {
    OcelStore::new(StateDb::open(std::path::Path::new(":memory:")).unwrap())
}

fn discovered_rows(store: &OcelStore, domain: &str) -> Vec<(String, f64, String)> {
    let conn = store.db().conn();
    let mut stmt = conn
        .prepare(
            "SELECT id, discovered_fitness, status FROM discovered_workflows WHERE domain = ?1",
        )
        .unwrap();
    stmt.query_map(rusqlite::params![domain], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    })
    .unwrap()
    .collect::<Result<Vec<_>, _>>()
    .unwrap()
}

#[test]
fn threshold_met_and_no_declared_fitness_persists_a_pending_suggestion() {
    let store = open_store();
    seed_domain(&store, "billing", ADMITTED_SCOPES_THRESHOLD as usize);
    let dw = discover_for_domain("billing", &store)
        .expect("discovery runs")
        .expect("fitness lift over declared 0.0 must produce a suggestion");
    assert_eq!(dw.domain, "billing");
    assert_eq!(dw.status, "pending");
    assert_eq!(dw.declared_fitness, 0.0);
    assert!(dw.discovered_fitness > FITNESS_LIFT && dw.discovered_fitness <= 1.0);
    assert!(
        dw.powl_string.starts_with("DISCOVERED_DFG{transitions="),
        "{}",
        dw.powl_string
    );

    let rows = discovered_rows(&store, "billing");
    assert_eq!(rows.len(), 1, "exactly one persisted suggestion");
    assert_eq!(rows[0].0, dw.id);
    assert_eq!(rows[0].2, "pending");
    assert!((rows[0].1 - dw.discovered_fitness).abs() < 1e-12);

    assert_eq!(record_feedback(&store, &dw.id, true).unwrap(), "accepted");
    assert_eq!(discovered_rows(&store, "billing")[0].2, "accepted");
}

#[test]
fn one_scope_below_threshold_discovers_nothing() {
    let store = open_store();
    seed_domain(&store, "billing", ADMITTED_SCOPES_THRESHOLD as usize - 1);
    assert!(discover_for_domain("billing", &store).unwrap().is_none());
    assert!(discovered_rows(&store, "billing").is_empty());
}

#[test]
fn no_lift_over_declared_fitness_discovers_nothing() {
    let store = open_store();
    seed_domain(&store, "billing", ADMITTED_SCOPES_THRESHOLD as usize);
    {
        let conn = store.db().conn();
        conn.execute(
            "INSERT INTO conformance_runs (run_id, scope_token, workflow_class, fitness, verdict, ran_at)
             VALUES ('run-1', 'scope-billing-0', 'billing', 0.99, 'pass', datetime('now'))",
            [],
        )
        .unwrap();
    }
    assert!(
        discover_for_domain("billing", &store).unwrap().is_none(),
        "discovered fitness cannot exceed 0.99 + FITNESS_LIFT"
    );
    assert!(discovered_rows(&store, "billing").is_empty());
}

#[test]
fn other_domains_do_not_leak_into_discovery() {
    let store = open_store();
    seed_domain(&store, "billing", ADMITTED_SCOPES_THRESHOLD as usize);
    assert!(discover_for_domain("shipping", &store).unwrap().is_none());
    assert!(discovered_rows(&store, "shipping").is_empty());
}
