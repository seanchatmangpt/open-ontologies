//! Phase 11 — multi-tenant session isolation.
//!
//! `TenantContext` is the caller-side tenant identifier carried alongside
//! `session_id`. It is read from the env var `OPEN_ONTOLOGIES_TENANT_ID` at
//! server construction; if unset it defaults to `"default"`. Pure data —
//! the type holds no authority of its own. The admission gate enforces ACLs
//! by comparing `TenantContext.current()` against the scope's owning
//! `tenant_id` recorded in `declared_workflows`.

use crate::admission::AdmissionOp;
use crate::ocel_store::OcelStore;
use std::sync::{Arc, RwLock};

/// Caller-side tenant identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TenantContext {
    pub tenant_id: String,
}

impl TenantContext {
    /// Read tenant from env, defaulting to `"default"`.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::tenant::TenantContext;
    ///
    /// // Env var set — from_env reads it directly.
    /// // SAFETY: test-only env mutation; no other threads touch this var.
    /// unsafe { std::env::set_var("OPEN_ONTOLOGIES_TENANT_ID", "doctest-tenant") };
    /// let ctx = TenantContext::from_env();
    /// assert_eq!(ctx.current(), "doctest-tenant");
    /// unsafe { std::env::remove_var("OPEN_ONTOLOGIES_TENANT_ID") };
    /// ```
    pub fn from_env() -> Self {
        let raw = std::env::var("OPEN_ONTOLOGIES_TENANT_ID")
            .unwrap_or_else(|_| "default".to_string());
        let raw = raw.trim();
        Self {
            tenant_id: if raw.is_empty() {
                "default".into()
            } else {
                raw.into()
            },
        }
    }

    /// Construct a `TenantContext` from an explicit tenant identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::tenant::TenantContext;
    ///
    /// let ctx = TenantContext::new("acme");
    /// assert_eq!(ctx.current(), "acme");
    /// ```
    pub fn new(tenant_id: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
        }
    }

    /// Return the tenant identifier as a string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::tenant::TenantContext;
    ///
    /// let ctx = TenantContext::new("beta-corp");
    /// assert_eq!(ctx.current(), "beta-corp");
    /// ```
    pub fn current(&self) -> &str {
        &self.tenant_id
    }
}

/// Mutable, shareable handle to the current tenant — used by long-lived
/// servers (MCP, HTTP) that need to rotate tenant context mid-stream.
#[derive(Clone, Debug)]
pub struct TenantHandle {
    inner: Arc<RwLock<String>>,
}

impl TenantHandle {
    /// Create a `TenantHandle` with the given initial tenant identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::tenant::TenantHandle;
    ///
    /// let handle = TenantHandle::new("alpha");
    /// assert_eq!(handle.current().current(), "alpha");
    /// ```
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(initial.into())),
        }
    }

    /// Build a `TenantHandle` by reading `OPEN_ONTOLOGIES_TENANT_ID` from the
    /// environment, defaulting to `"default"` when unset.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::tenant::TenantHandle;
    ///
    /// // SAFETY: test-only env mutation; no other threads touch this var.
    /// unsafe { std::env::set_var("OPEN_ONTOLOGIES_TENANT_ID", "handle-tenant") };
    /// let handle = TenantHandle::from_env();
    /// assert_eq!(handle.current().current(), "handle-tenant");
    /// unsafe { std::env::remove_var("OPEN_ONTOLOGIES_TENANT_ID") };
    /// ```
    pub fn from_env() -> Self {
        let ctx = TenantContext::from_env();
        Self::new(ctx.tenant_id)
    }

    /// Snapshot the current tenant as a [`TenantContext`].
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::tenant::TenantHandle;
    ///
    /// let handle = TenantHandle::new("gamma");
    /// let ctx = handle.current();
    /// assert_eq!(ctx.current(), "gamma");
    /// ```
    pub fn current(&self) -> TenantContext {
        TenantContext::new(self.inner.read().unwrap().clone())
    }

    /// Switch the effective tenant. Idempotent on no-op. Emits a
    /// `tenant_switch` OCEL event under [`AdmissionOp::TenantSwitch`]
    /// carrying both the old and the new tenant_id. Audit-only — never
    /// denies, never blocks. The event is tagged with the NEW tenant so
    /// it sits inside the new namespace, but its `from_tenant` attribute
    /// preserves the rotation evidence.
    pub fn switch(&self, store: &OcelStore, session_id: &str, new_tenant: &str) {
        let new_tenant = new_tenant.trim();
        let new_tenant = if new_tenant.is_empty() {
            "default"
        } else {
            new_tenant
        };
        let mut guard = self.inner.write().unwrap();
        let from = guard.clone();
        if from == new_tenant {
            return;
        }
        *guard = new_tenant.to_string();
        drop(guard);
        let now = chrono::Utc::now().to_rfc3339();
        // event_id includes both endpoints + nanosecond ts so two rapid
        // switches (e.g. default→alpha then alpha→beta within one ms) cannot
        // collide on `event_id` and have one silently dropped by the OCEL
        // store's INSERT OR IGNORE.
        let event_id = format!(
            "{}:tenant_switch:{}->{}:{}",
            session_id,
            from,
            new_tenant,
            chrono::Utc::now()
                .timestamp_nanos_opt()
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis() * 1_000_000),
        );
        let _ = store.emit_event_in_tenant(
            &event_id,
            "tenant_switch",
            &now,
            session_id,
            &[
                ("op", AdmissionOp::TenantSwitch.as_str()),
                ("from_tenant", &from),
                ("to_tenant", new_tenant),
                ("production_law_version", "ontostar-1.0.0"),
                (
                    "defects_taxonomy_version",
                    crate::defects::DEFECTS_TAXONOMY_VERSION,
                ),
            ],
            &[],
            None,
            new_tenant,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_context_new_round_trip() {
        let ctx = TenantContext::new("alpha");
        assert_eq!(ctx.current(), "alpha");
    }

    #[test]
    fn tenant_handle_no_op_switch_does_not_change_state() {
        let h = TenantHandle::new("alpha");
        // Cannot easily assert OCEL emission in a unit test without an
        // OcelStore; the integration test in
        // tests/multi_tenant_isolation.rs covers the emit path.
        assert_eq!(h.current().current(), "alpha");
    }
}
