//! R7 WA2 — A2 V1 Receipt-Chain Verifier sabotage test (corrupt key_valid_at).
//!
//! Doctrine: ZERO LLM in this code path. The verifier must produce a
//! deterministic verdict from `(receipt_row, trusted_keys_history_row)`
//! alone. This test sabotages the receipt-row side: rewrites
//! `key_valid_at` to a value that does NOT appear in `trusted_keys_history`,
//! then runs `tick_once()` and asserts:
//!
//!   1. exactly one OCEL `verifier_failure` row was emitted with the
//!      receipt's hash in `event_id` (`verifier_failure::<hash>`);
//!   2. `retention_paused_until` is now > Utc::now().timestamp() (the
//!      worker `fetch_max`'d the pause atomic);
//!   3. `tick()` returned `Ok` (the worker did not panic).
//!
//! Sabotage variants we DON'T cover here:
//!   - Corrupting the body hash → falls under Cell8 A5 chain-link gate
//!     (separate test in `tests/key_rotation.rs`).
//!   - Empty `key_valid_at` → that's the legacy unsigned path and is
//!     deliberately treated as `Ok` (covered indirectly by the
//!     `verifier_idempotent` test).

use open_ontologies::config::VerifierConfig;
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::state::StateDb;
use open_ontologies::verifier_worker::VerifierWorker;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tempfile::tempdir;

fn fresh_db() -> (StateDb, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let path = dir.path().join("verifier.db");
    let db = StateDb::open(&path).expect("open StateDb");
    (db, dir)
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Insert a receipt with a `key_valid_at` that is NOT present in
/// `trusted_keys_history`. The verifier should fail it as `UnknownKey`.
fn insert_corrupt_receipt(db: &StateDb, hash: &str, sequence: i64) {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO receipts (
            receipt_hash, scope_token, session_id,
            artifact_hash, declared_powl_hash, ocel_canonical_hash,
            gate_config_hash, prior_receipt_hash,
            production_law_version, granted_at, sequence, tenant_id,
            key_valid_at
         ) VALUES (?1,'scope-x','session-x',?1,?1,?1,?1,NULL,'ontostar-1.0.0',?2,?3,'default','BOGUS_NOT_IN_HISTORY')",
        rusqlite::params![hash, now_iso(), sequence],
    )
    .unwrap();
}

#[test]
fn corrupt_receipt_emits_verifier_failure_and_pauses_retention() {
    let (db, _g) = fresh_db();
    let ocel = Arc::new(OcelStore::new(db.clone()));
    let pause = Arc::new(AtomicI64::new(0));

    // ── 1. Insert one corrupt receipt with bogus key_valid_at ────────
    let receipt_hash = "deadbeef".repeat(8); // 64-char hex
    insert_corrupt_receipt(&db, &receipt_hash, 1);

    // ── 2. Tick the verifier (synchronously) ─────────────────────────
    let cfg = VerifierConfig {
        enabled: true,
        tick_secs: 1,
        batch_limit: 100,
        pause_retention_on_failure: true,
        pause_minutes_on_failure: 60,
        andon_on_failure: true,
        max_lookback_days: None,
    };
    let worker = VerifierWorker::new(db.clone(), ocel.clone(), cfg, pause.clone());
    let report = worker.tick().expect("tick must not panic");

    assert_eq!(report.scanned, 1, "should scan exactly one receipt");
    assert_eq!(
        report.failures, 1,
        "corrupt key_valid_at must produce exactly one failure"
    );
    assert_eq!(report.warnings, 0, "no warnings expected");

    // ── 3. Verify the OCEL row was emitted with deterministic event_id
    let event_id = format!("verifier_failure::{}", receipt_hash);
    let count: i64 = ocel
        .db()
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM ocel_events WHERE event_id = ?1
                 AND event_type = 'verifier_failure'",
            rusqlite::params![event_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        count, 1,
        "exactly one verifier_failure OCEL row with deterministic event_id"
    );

    // Confirm at least the `kind` attribute landed (idempotent attr write).
    let kind: String = ocel
        .db()
        .conn()
        .query_row(
            "SELECT value FROM ocel_event_attrs
                 WHERE event_id = ?1 AND name = 'kind'",
            rusqlite::params![event_id],
            |r| r.get(0),
        )
        .expect("kind attribute should be present");
    assert_eq!(kind, "unknown_key", "verdict must be UnknownKey");

    // ── 4. retention_paused_until must have advanced ─────────────────
    let now = chrono::Utc::now().timestamp();
    let paused_until = pause.load(Ordering::Relaxed);
    assert!(
        paused_until > now,
        "retention_paused_until ({paused_until}) must exceed now ({now})"
    );
    // Pause window should be roughly +60 minutes (3600s); allow 5s slack.
    assert!(
        paused_until - now > 3500 && paused_until - now <= 3600,
        "pause window should be ~60 min from now; got {} s",
        paused_until - now
    );

    // ── 5. cursor must have advanced past the corrupt row ───────────
    let cursor = worker.last_verified_seq.load(Ordering::Relaxed);
    assert_eq!(cursor, 1, "cursor must advance past sequence 1");
}

#[test]
fn corrupt_receipt_with_pause_disabled_does_not_pause_retention() {
    let (db, _g) = fresh_db();
    let ocel = Arc::new(OcelStore::new(db.clone()));
    let pause = Arc::new(AtomicI64::new(0));

    let receipt_hash = "cafebabe".repeat(8);
    insert_corrupt_receipt(&db, &receipt_hash, 1);

    // pause_retention_on_failure = false — failure still emits OCEL but
    // does NOT advance the pause atomic.
    let cfg = VerifierConfig {
        enabled: true,
        tick_secs: 1,
        batch_limit: 100,
        pause_retention_on_failure: false,
        pause_minutes_on_failure: 60,
        andon_on_failure: false,
        max_lookback_days: None,
    };
    let worker = VerifierWorker::new(db.clone(), ocel.clone(), cfg, pause.clone());
    let report = worker.tick().expect("tick must not panic");
    assert_eq!(report.failures, 1);
    assert_eq!(
        pause.load(Ordering::Relaxed),
        0,
        "pause atomic must remain 0 when pause_retention_on_failure=false"
    );
}
