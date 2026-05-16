//! Workflow scope: open / close persisted to `declared_workflows`.
//!
//! # Examples
//!
//! Open a scope from the built-in catalog, then close it:
//!
//! ```
//! use open_ontologies::state::StateDb;
//! use open_ontologies::workflows::scope::WorkflowScope;
//!
//! let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
//! let scope = WorkflowScope::new(&db, "doc-session");
//!
//! let token = scope.open(Some("OntologyAuthoring"), None, None).unwrap();
//! assert!(!token.is_empty());
//!
//! let row = scope.get(&token).unwrap().unwrap();
//! assert_eq!(row.status, "open");
//!
//! scope.close(&token).unwrap();
//! let row = scope.get(&token).unwrap().unwrap();
//! assert_eq!(row.status, "closed");
//! ```

use crate::defects::DefectClass;
use crate::state::StateDb;
use crate::workflows::builtin;

/// Result of opening a scope: typed errors map to [`DefectClass`].
#[derive(Debug)]
pub enum ScopeError {
    /// Caller supplied neither a known catalog `name` nor an inline `powl`.
    Defect(DefectClass),
    /// SQL persistence failed.
    Storage(String),
}

impl From<rusqlite::Error> for ScopeError {
    fn from(e: rusqlite::Error) -> Self {
        ScopeError::Storage(e.to_string())
    }
}

/// Snapshot of an open scope.
#[derive(Debug, Clone)]
pub struct ScopeRow {
    pub scope_token: String,
    pub session_id: String,
    pub name: String,
    pub powl_string: String,
    pub powl_hash: String,
    pub status: String,
}

/// Workflow scope manager — persists declarations to `declared_workflows`.
pub struct WorkflowScope<'a> {
    db: &'a StateDb,
    session_id: &'a str,
}

impl<'a> WorkflowScope<'a> {
    pub fn new(db: &'a StateDb, session_id: &'a str) -> Self {
        Self { db, session_id }
    }

    /// Open a scope. Returns the scope token (ULID).
    ///
    /// - If `scope_token` is provided, it is reused (idempotent re-open).
    /// - If `name` resolves to a built-in catalog entry, its POWL string is used.
    /// - If `powl` is provided, it is used directly (and `name` defaults to "custom").
    /// - At least one of `name` or `powl` must be supplied.
    ///
    /// # Examples
    ///
    /// Open by catalog name:
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::workflows::scope::WorkflowScope;
    ///
    /// let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// let scope = WorkflowScope::new(&db, "sess-open");
    /// let token = scope.open(Some("Codegen"), None, None).unwrap();
    /// assert!(!token.is_empty());
    /// ```
    ///
    /// Open with an inline POWL string (name defaults to "custom"):
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::workflows::scope::WorkflowScope;
    ///
    /// let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// let scope = WorkflowScope::new(&db, "sess-inline");
    /// let token = scope.open(None, Some("SEQ(a, b, c)"), None).unwrap();
    /// let row = scope.get(&token).unwrap().unwrap();
    /// assert_eq!(row.name, "custom");
    /// assert_eq!(row.powl_string, "SEQ(a, b, c)");
    /// ```
    pub fn open(
        &self,
        name: Option<&str>,
        powl: Option<&str>,
        scope_token: Option<&str>,
    ) -> Result<String, ScopeError> {
        self.open_in_tenant(name, powl, scope_token, "default")
    }

    /// Phase 11 — tenant-aware variant of [`open`]. Tags the new
    /// `declared_workflows` row with `tenant_id` so cross-tenant access
    /// can be denied at admission time.
    pub fn open_in_tenant(
        &self,
        name: Option<&str>,
        powl: Option<&str>,
        scope_token: Option<&str>,
        tenant_id: &str,
    ) -> Result<String, ScopeError> {
        // Resolve name + powl_string.
        let (resolved_name, powl_string, alphabet) = match (name, powl) {
            (Some(n), Some(p)) => (n.to_string(), p.to_string(), Vec::<String>::new()),
            (Some(n), None) => match builtin::by_name(n) {
                Some(b) => (
                    b.name.to_string(),
                    b.powl_string.to_string(),
                    b.alphabet.iter().map(|s| s.to_string()).collect(),
                ),
                None => {
                    return Err(ScopeError::Defect(DefectClass::CapabilityZero));
                }
            },
            (None, Some(p)) => ("custom".to_string(), p.to_string(), Vec::new()),
            (None, None) => {
                return Err(ScopeError::Defect(DefectClass::DeadParameter {
                    param: "name|powl".into(),
                }));
            }
        };

        let token = scope_token
            .map(|s| s.to_string())
            .unwrap_or_else(|| ulid::Ulid::new().to_string());

        let powl_hash = blake3::hash(powl_string.as_bytes()).to_hex().to_string();
        let alphabet_json =
            serde_json::to_string(&alphabet).unwrap_or_else(|_| "[]".to_string());
        let now = chrono::Utc::now().to_rfc3339();

        let conn = self.db.conn();
        conn.execute(
            "INSERT OR REPLACE INTO declared_workflows
                (scope_token, session_id, name, powl_string, powl_hash, alphabet_json,
                 declared_at, closed_at, status, tenant_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, 'open', ?8)",
            rusqlite::params![
                &token,
                self.session_id,
                &resolved_name,
                &powl_string,
                &powl_hash,
                &alphabet_json,
                &now,
                tenant_id,
            ],
        )?;
        Ok(token)
    }

    /// Close a scope by writing `closed_at` and flipping status to `closed`.
    /// Returns `Err(ScopeUnclosed)` if the token is not found.
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::workflows::scope::{WorkflowScope, ScopeError};
    /// use open_ontologies::defects::DefectClass;
    ///
    /// let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// let scope = WorkflowScope::new(&db, "sess-close");
    /// let token = scope.open(Some("Alignment"), None, None).unwrap();
    ///
    /// // First close succeeds.
    /// scope.close(&token).unwrap();
    ///
    /// // Closing again yields ScopeUnclosed.
    /// match scope.close(&token) {
    ///     Err(ScopeError::Defect(DefectClass::ScopeUnclosed)) => {}
    ///     other => panic!("expected ScopeUnclosed, got {:?}", other),
    /// }
    /// ```
    pub fn close(&self, scope_token: &str) -> Result<(), ScopeError> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.db.conn();
        let n = conn.execute(
            "UPDATE declared_workflows
                SET closed_at = ?1, status = 'closed'
              WHERE scope_token = ?2 AND status = 'open'",
            rusqlite::params![&now, scope_token],
        )?;
        if n == 0 {
            return Err(ScopeError::Defect(DefectClass::ScopeUnclosed));
        }
        Ok(())
    }

    /// Return the most recently declared (and not-yet-closed) scope row for
    /// this session, if any. Used by Stream 3 admission gates that operate
    /// on the implicit "current scope".
    ///
    /// # Examples
    ///
    /// ```
    /// use open_ontologies::state::StateDb;
    /// use open_ontologies::workflows::scope::WorkflowScope;
    ///
    /// let db = StateDb::open(std::path::Path::new(":memory:")).unwrap();
    /// let scope = WorkflowScope::new(&db, "sess-latest");
    ///
    /// // No open scope yet.
    /// assert!(scope.latest_open().unwrap().is_none());
    ///
    /// // After opening one, latest_open returns it.
    /// let token = scope.open(Some("DataExtension"), None, None).unwrap();
    /// let row = scope.latest_open().unwrap().unwrap();
    /// assert_eq!(row.scope_token, token);
    /// assert_eq!(row.status, "open");
    ///
    /// // After closing, latest_open returns None again.
    /// scope.close(&token).unwrap();
    /// assert!(scope.latest_open().unwrap().is_none());
    /// ```
    pub fn latest_open(&self) -> Result<Option<ScopeRow>, ScopeError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT scope_token, session_id, name, powl_string, powl_hash, status
               FROM declared_workflows
              WHERE session_id = ?1 AND status = 'open'
              ORDER BY declared_at DESC
              LIMIT 1",
        )?;
        let mut rows = stmt.query(rusqlite::params![self.session_id])?;
        if let Some(r) = rows.next()? {
            Ok(Some(ScopeRow {
                scope_token: r.get(0)?,
                session_id: r.get(1)?,
                name: r.get(2)?,
                powl_string: r.get(3)?,
                powl_hash: r.get(4)?,
                status: r.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Return the most-recent scope row for this session regardless of status.
    pub fn latest_any(&self) -> Result<Option<ScopeRow>, ScopeError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT scope_token, session_id, name, powl_string, powl_hash, status
               FROM declared_workflows
              WHERE session_id = ?1
              ORDER BY declared_at DESC
              LIMIT 1",
        )?;
        let mut rows = stmt.query(rusqlite::params![self.session_id])?;
        if let Some(r) = rows.next()? {
            Ok(Some(ScopeRow {
                scope_token: r.get(0)?,
                session_id: r.get(1)?,
                name: r.get(2)?,
                powl_string: r.get(3)?,
                powl_hash: r.get(4)?,
                status: r.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Phase 11 — fetch the `tenant_id` of a declared scope. Returns
    /// `"default"` for legacy rows that predate the column. Returns `None`
    /// if no row exists.
    pub fn tenant_for(&self, scope_token: &str) -> Result<Option<String>, ScopeError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT tenant_id FROM declared_workflows WHERE scope_token = ?1",
        )?;
        let mut rows = stmt.query(rusqlite::params![scope_token])?;
        if let Some(r) = rows.next()? {
            Ok(Some(r.get::<_, String>(0)?))
        } else {
            Ok(None)
        }
    }

    /// Fetch a scope row by token (for tests / inspection).
    pub fn get(&self, scope_token: &str) -> Result<Option<ScopeRow>, ScopeError> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT scope_token, session_id, name, powl_string, powl_hash, status
               FROM declared_workflows
              WHERE scope_token = ?1",
        )?;
        let mut rows = stmt.query(rusqlite::params![scope_token])?;
        if let Some(r) = rows.next()? {
            Ok(Some(ScopeRow {
                scope_token: r.get(0)?,
                session_id: r.get(1)?,
                name: r.get(2)?,
                powl_string: r.get(3)?,
                powl_hash: r.get(4)?,
                status: r.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn fresh_db() -> StateDb {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ontostar-scope-test.db");
        // Leak the tempdir so the file outlives this fn for the test.
        std::mem::forget(dir);
        StateDb::open(&path).expect("open StateDb")
    }

    #[test]
    fn open_then_close_roundtrip_persists_row() {
        let db = fresh_db();
        let session = "test-session-1";
        let scope = WorkflowScope::new(&db, session);

        let token = scope
            .open(Some("OntologyAuthoring"), None, None)
            .expect("open by catalog name");

        // Row exists, status = open.
        let row = scope.get(&token).unwrap().expect("row present after open");
        assert_eq!(row.session_id, session);
        assert_eq!(row.name, "OntologyAuthoring");
        assert_eq!(row.status, "open");
        assert!(!row.powl_hash.is_empty());
        // R1 rewrote catalog to wasm4pm grammar (PO=(...)). Just assert "load" is in the alphabet.
        assert!(row.powl_string.contains("load"));

        // Close flips status.
        scope.close(&token).expect("close");
        let row = scope.get(&token).unwrap().expect("row still present");
        assert_eq!(row.status, "closed");

        // Closing twice yields ScopeUnclosed defect.
        match scope.close(&token) {
            Err(ScopeError::Defect(DefectClass::ScopeUnclosed)) => {}
            other => panic!("expected ScopeUnclosed, got {:?}", other),
        }
    }

    #[test]
    fn open_inline_powl_uses_custom_name() {
        let db = fresh_db();
        let scope = WorkflowScope::new(&db, "s2");
        let token = scope
            .open(None, Some("SEQ(a, b)"), None)
            .expect("open by inline powl");
        let row = scope.get(&token).unwrap().unwrap();
        assert_eq!(row.name, "custom");
        assert_eq!(row.powl_string, "SEQ(a, b)");
    }

    #[test]
    fn open_unknown_name_yields_capability_zero() {
        let db = fresh_db();
        let scope = WorkflowScope::new(&db, "s3");
        match scope.open(Some("NotInCatalog"), None, None) {
            Err(ScopeError::Defect(DefectClass::CapabilityZero)) => {}
            other => panic!("expected CapabilityZero, got {:?}", other),
        }
    }

    #[test]
    fn open_with_no_name_or_powl_is_dead_parameter() {
        let db = fresh_db();
        let scope = WorkflowScope::new(&db, "s4");
        match scope.open(None, None, None) {
            Err(ScopeError::Defect(DefectClass::DeadParameter { param })) => {
                assert_eq!(param, "name|powl");
            }
            other => panic!("expected DeadParameter, got {:?}", other),
        }
    }
}
