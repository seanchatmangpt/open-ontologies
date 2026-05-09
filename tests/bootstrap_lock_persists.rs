//! R5 WC-1 — §28 HiddenWIP closure for the bootstrap window.
//!
//! Counterfactual: once a non-`seed-v0` receipt has been persisted, the
//! `bootstrap_lock` row is laid down at the DB level. The bootstrap window
//! is **permanently** closed:
//!
//!   * The R4 WD `RetentionWorker` cannot prune the lock (it is not in any
//!     pruner's WHERE clause). Even with `*_days = 0` (cutoff = now),
//!     `bootstrap_lock` survives the cascade.
//!   * The advisory `OPEN_ONTOLOGIES_BOOTSTRAP_MODE=1` env var is IGNORED
//!     once the lock exists (lock-row precedence in
//!     `BootstrapState::is_bootstrap`).
//!
//! Pre-fix behaviour: `is_bootstrap` reread the env var fresh per call AND
//! re-counted receipts; if the retention worker pruned all production
//! receipts, the count returned to 0 and the bootstrap window silently
//! re-opened — a §27 EscapeRoute leak.
//!
//! This test would FAIL against pre-fix `bootstrap.rs` because (a) there
//! was no `bootstrap_lock` table and (b) `is_bootstrap` would return true
//! after retention pruning re-zeroed the receipt count.

use open_ontologies::bootstrap::BootstrapState;
use open_ontologies::config::RetentionConfig;
use open_ontologies::production_record::ProductionRecord;
use open_ontologies::receipts::{self, Receipt};
use open_ontologies::retention::RetentionWorker;
use open_ontologies::state::StateDb;

fn build_db() -> (tempfile::TempDir, StateDb) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("server.db")).unwrap();
    (tmp, db)
}

fn build_test_receipt(law_version: &str, scope: &str) -> Receipt {
    let record = ProductionRecord {
        artifact_hash: [0u8; 32],
        scope_token: scope.to_string(),
        declared_powl_hash: [0u8; 32],
        ocel_canonical_hash: [0u8; 32],
        conformance_run_id: "test-run".to_string(),
        gate_config_hash: [0u8; 32],
        production_law_version: law_version.to_string(),
        defects_taxonomy_version: "ontostar-defects-4.4.0".to_string(),
        gates_passed: vec![],
        gates_refused: vec![],
        prior_receipt: None,
        signature: None,
        signing_key_fpr: None,
    };
    receipts::build(record)
}

fn count_lock_rows(db: &StateDb) -> i64 {
    db.conn()
        .query_row("SELECT COUNT(*) FROM bootstrap_lock", [], |r| r.get(0))
        .unwrap_or(-1)
}

#[test]
fn bootstrap_lock_inserted_on_first_non_seed_receipt() {
    let (_tmp, db) = build_db();

    // Fresh DB: no lock row, `is_bootstrap` is true (no non-seed receipts yet).
    assert_eq!(count_lock_rows(&db), 0);
    assert!(
        BootstrapState::is_bootstrap(&db),
        "fresh DB must report bootstrap"
    );

    // Persist a seed receipt — must NOT close the window.
    let seed = build_test_receipt("seed-v0", "scope-seed");
    receipts::persist_with_tenant(&seed, &db, "session-A", "tenant-A").unwrap();
    assert_eq!(count_lock_rows(&db), 0, "seed receipt must not lock");
    assert!(
        BootstrapState::is_bootstrap(&db),
        "still in bootstrap after seed-v0 only"
    );

    // First non-seed receipt — auto-locks at the DB level.
    let prod = build_test_receipt("ontostar-1.0.0", "scope-prod-1");
    receipts::persist_with_tenant(&prod, &db, "session-B", "tenant-prod").unwrap();
    assert_eq!(count_lock_rows(&db), 1, "non-seed receipt must lock");
    assert!(
        !BootstrapState::is_bootstrap(&db),
        "lock row must close the window"
    );
}

#[test]
fn bootstrap_lock_idempotent_under_repeated_non_seed_receipts() {
    let (_tmp, db) = build_db();

    // Persist 10 non-seed receipts — `INSERT OR IGNORE` keeps lock COUNT = 1.
    for i in 0..10 {
        let r = build_test_receipt("ontostar-1.0.0", &format!("scope-{i}"));
        receipts::persist_with_tenant(&r, &db, &format!("session-{i}"), "tenant-prod").unwrap();
    }
    assert_eq!(
        count_lock_rows(&db),
        1,
        "INSERT OR IGNORE must keep exactly one lock row"
    );

    // The original locked_by must be preserved (first writer wins).
    let locked_by: String = db
        .conn()
        .query_row(
            "SELECT locked_by FROM bootstrap_lock WHERE id = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(locked_by, "tenant-prod");
}

#[tokio::test]
async fn bootstrap_lock_survives_retention_pruning() {
    let (_tmp, db) = build_db();

    // Lay down the lock via a non-seed receipt.
    let prod = build_test_receipt("ontostar-1.0.0", "scope-prod");
    receipts::persist_with_tenant(&prod, &db, "session-prod", "tenant-prod").unwrap();
    assert_eq!(count_lock_rows(&db), 1);

    // Run retention with EVERY pruner aggressive (cutoff = now). If the
    // lock were ever added to a pruner's WHERE clause, it would be wiped.
    let worker = RetentionWorker::new(
        db.clone(),
        RetentionConfig {
            poll_interval_secs: 1,
            ocel_days: 0,
            lineage_days: 0,
            conformance_days: 0,
            revocation_grace_days: 0,
            receipt_files_days: 0,
            exemplar_days: 0,
            feedback_days: 0,
            archive_path: None,
            hot_receipt_days: 0,
        },
    );
    let _report = worker.tick().expect("tick must succeed");

    // The lock survives — that is the load-bearing assertion.
    assert_eq!(
        count_lock_rows(&db),
        1,
        "bootstrap_lock must NOT be pruned by RetentionWorker"
    );
    assert!(
        !BootstrapState::is_bootstrap(&db),
        "post-retention `is_bootstrap` must STILL be false (lock wins)"
    );
}

#[test]
fn bootstrap_lock_supersedes_env_override() {
    let (_tmp, db) = build_db();

    // Lay the lock down.
    let prod = build_test_receipt("ontostar-1.0.0", "scope-prod-env");
    receipts::persist_with_tenant(&prod, &db, "session-env", "tenant-env").unwrap();

    // Even if an operator sets the advisory env var, the lock wins.
    let _guard = ScopedEnv::set("OPEN_ONTOLOGIES_BOOTSTRAP_MODE", "1");
    assert!(
        !BootstrapState::is_bootstrap(&db),
        "env=1 must NOT re-open the window once the lock exists"
    );
}

/// Scoped env var setter — restores the previous value (or removes the var
/// if it was unset) when the guard is dropped. Uses `unsafe` because Rust
/// 1.78+ marks `set_var`/`remove_var` as unsafe under multi-threaded
/// scenarios; tests in the same binary may race, so callers should set
/// `--test-threads=1` if they actually depend on the env var across tests.
struct ScopedEnv {
    key: String,
    prev: Option<String>,
}

impl ScopedEnv {
    fn set(key: &str, val: &str) -> Self {
        let prev = std::env::var(key).ok();
        // SAFETY: tests using this guard must run single-threaded if they
        // race on the same env var. The two tests in this file that
        // touch OPEN_ONTOLOGIES_BOOTSTRAP_MODE are independent of others.
        unsafe { std::env::set_var(key, val) };
        Self {
            key: key.to_string(),
            prev,
        }
    }
}

impl Drop for ScopedEnv {
    fn drop(&mut self) {
        // SAFETY: see Self::set.
        unsafe {
            match &self.prev {
                Some(v) => std::env::set_var(&self.key, v),
                None => std::env::remove_var(&self.key),
            }
        }
    }
}
