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

/// Type-level marker for the bootstrap window.
pub struct BootstrapState;

impl BootstrapState {
    /// True iff the system is still in its bootstrap window. See module
    /// docs for the precedence order.
    ///
    /// Lock-row precedence is absolute: once the lock exists, the
    /// bootstrap window is permanently closed. Use `onto_bootstrap_unlock`
    /// (admin-only, R5 WC-2) for last-resort recovery.
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
