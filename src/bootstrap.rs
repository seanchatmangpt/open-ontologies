//! Bootstrap-window admission state.
//!
//! Some operator-tier handlers — notably `onto_exemplar_seed` — must be
//! callable BEFORE any production receipts have been admitted. They cannot
//! be gated by `evaluate_admission()` because there is no scope to admit
//! against, but they MUST NOT be allowed to mutate the store after
//! production traffic has begun.
//!
//! # R5 WC-1 — §28 HiddenWIP closure
//!
//! The previous implementation read `OPEN_ONTOLOGIES_BOOTSTRAP_MODE` and
//! the receipt count fresh on every call. Both are volatile:
//!
//!   * The env var can be re-set after startup (TOCTOU surface).
//!   * The receipt count can drop back to zero if the R4 WD retention
//!     worker prunes all non-seed receipts, silently re-opening the
//!     window — a §27 EscapeRoutes leak.
//!
//! `BootstrapState::is_bootstrap(db)` now consults a DB-level lock row
//! that is inserted (idempotently) on the first non-`seed-v0` receipt:
//!
//!   1. If the `bootstrap_lock` row exists → window is **CLOSED** (returns
//!      `false`). The env var is ignored once the lock has been laid down.
//!   2. Else if `OPEN_ONTOLOGIES_BOOTSTRAP_MODE=1` → advisory open
//!      (returns `true`). Useful for fresh-image bring-up and integration
//!      tests. Once a non-seed receipt lands the lock supersedes the env.
//!   3. Else if the `receipts` table contains zero rows whose
//!      `production_law_version != 'seed-v0'` → bootstrap (returns `true`).
//!   4. Otherwise → closed (returns `false`).
//!
//! See `src/receipts.rs::persist_with_tenant_in_tx` for the auto-lock
//! insertion site. See `tests/bootstrap_lock_persists.rs` for the
//! retention-survival counterfactual.
//!
//! The check is read-only and idempotent. Callers are expected to refuse
//! the request with [`crate::defects::DefectClass::BootstrapClosed`] when
//! `is_bootstrap` returns `false`.

use crate::state::StateDb;

/// Minimum `granted_at_chain` length required after the bootstrap lock is
/// set (R8-1 chain-length gate). A chain shorter than this in a
/// post-bootstrap admission with `prior_tenant_receipt_count > 0` raises
/// [`crate::defects::DefectClass::BootstrapChainTooShort`].
///
/// ```
/// use open_ontologies::bootstrap::MIN_BOOTSTRAP_CHAIN_LENGTH;
///
/// // The R8 gate requires at least two prior receipts.
/// assert_eq!(MIN_BOOTSTRAP_CHAIN_LENGTH, 2);
///
/// // A chain of length ≥ 2 passes the gate.
/// let chain = vec!["receipt-a".to_string(), "receipt-b".to_string()];
/// assert!(chain.len() >= MIN_BOOTSTRAP_CHAIN_LENGTH);
///
/// // A chain of length 1 would fail the gate.
/// let short_chain = vec!["only-one".to_string()];
/// assert!(short_chain.len() < MIN_BOOTSTRAP_CHAIN_LENGTH);
/// ```
pub const MIN_BOOTSTRAP_CHAIN_LENGTH: usize = 2;

/// Type-level marker for the bootstrap window.
///
/// All state is persisted in the [`StateDb`] SQLite database. The struct
/// itself carries no fields; it is a namespace for the associated functions.
///
/// ```
/// use open_ontologies::bootstrap::BootstrapState;
///
/// // BootstrapState is a zero-sized type — construction is always valid.
/// let _state = BootstrapState;
/// ```
pub struct BootstrapState;

impl BootstrapState {
    /// True iff the system is still in its bootstrap window. See module
    /// docs for the precedence order.
    ///
    /// Lock-row precedence is absolute: once the lock exists, the
    /// bootstrap window is permanently closed. Use `onto_bootstrap_unlock`
    /// (admin-only, R5 WC-2) for last-resort recovery.
    ///
    /// # Examples
    ///
    /// ## Fresh database — bootstrap window is open
    ///
    /// ```
    /// use open_ontologies::bootstrap::BootstrapState;
    /// use open_ontologies::state::StateDb;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// // No lock row, no non-seed receipts → window is open.
    /// assert!(BootstrapState::is_bootstrap(&db));
    /// ```
    ///
    /// ## Lock row present — window is permanently closed
    ///
    /// ```
    /// use open_ontologies::bootstrap::BootstrapState;
    /// use open_ontologies::state::StateDb;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    ///
    /// // Insert the lock row (normally done by `receipts::persist_with_tenant_in_tx`).
    /// db.conn().execute(
    ///     "INSERT OR IGNORE INTO bootstrap_lock (id, locked_at, locked_by) \
    ///      VALUES (1, datetime('now'), 'doctest')",
    ///     [],
    /// ).unwrap();
    ///
    /// // Lock supersedes everything — window is closed.
    /// assert!(!BootstrapState::is_bootstrap(&db));
    /// ```
    ///
    /// ## Non-seed receipt present (no lock yet) — window is closed
    ///
    /// ```
    /// use open_ontologies::bootstrap::BootstrapState;
    /// use open_ontologies::state::StateDb;
    /// use std::path::Path;
    ///
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    ///
    /// // Insert a non-seed-v0 receipt row without the lock.
    /// db.conn().execute(
    ///     "INSERT INTO receipts \
    ///      (receipt_hash, scope_token, artifact_hash, declared_powl_hash, \
    ///       ocel_canonical_hash, gate_config_hash, production_law_version, granted_at) \
    ///      VALUES ('h1','s1','a1','p1','o1','g1','v1','2026-01-01T00:00:00Z')",
    ///     [],
    /// ).unwrap();
    ///
    /// // Non-seed receipt → window is closed.
    /// assert!(!BootstrapState::is_bootstrap(&db));
    /// ```
    pub fn is_bootstrap(db: &StateDb) -> bool {
        let conn = db.conn();

        // 1. Lock row exists → CLOSED (one-shot DB-level enforcement).
        let locked: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM bootstrap_lock WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if locked > 0 {
            return false;
        }

        // 2. Advisory env-var override (only when the lock row is absent).
        if std::env::var("OPEN_ONTOLOGIES_BOOTSTRAP_MODE")
            .map(|v| v == "1")
            .unwrap_or(false)
        {
            return true;
        }

        // 3. No non-seed receipts yet → bootstrap.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM receipts WHERE production_law_version != 'seed-v0'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(i64::MAX);
        count == 0
    }
}
