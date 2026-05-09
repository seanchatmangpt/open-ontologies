//! Bootstrap-window admission state.
//!
//! Some operator-tier handlers — notably `onto_exemplar_seed` — must be
//! callable BEFORE any production receipts have been admitted. They cannot
//! be gated by `evaluate_admission()` because there is no scope to admit
//! against, but they MUST NOT be allowed to mutate the store after
//! production traffic has begun.
//!
//! `BootstrapState::is_bootstrap(db)` returns `true` iff:
//!   1. The environment variable `OPEN_ONTOLOGIES_BOOTSTRAP_MODE` is set
//!      to `"1"` (explicit override; intended for fresh-image bring-up
//!      and integration tests), OR
//!   2. The `receipts` table contains zero rows whose
//!      `production_law_version != 'seed-v0'` — i.e. nothing has been
//!      admitted under a real production law version yet.
//!
//! The check is read-only and idempotent. Callers are expected to refuse
//! the request with [`crate::defects::DefectClass::BootstrapClosed`] when
//! `is_bootstrap` returns `false`.
//!
//! See: `tests/round4_admission_op_bypass.rs`,
//!      `tests/round4_no_bypass_red_team.rs`.

use crate::state::StateDb;

/// Type-level marker for the bootstrap window.
pub struct BootstrapState;

impl BootstrapState {
    /// True iff the system is still in its bootstrap window — explicit env
    /// override OR no production receipts present in the store.
    ///
    /// Returns `true` defensively when the env var is set, even if the
    /// receipts query fails: an explicit operator override takes precedence
    /// over a transient DB error. When the env var is unset and the query
    /// fails, returns `false` (closed) — fail-safe.
    pub fn is_bootstrap(db: &StateDb) -> bool {
        if std::env::var("OPEN_ONTOLOGIES_BOOTSTRAP_MODE")
            .map(|v| v == "1")
            .unwrap_or(false)
        {
            return true;
        }
        let conn = db.conn();
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
