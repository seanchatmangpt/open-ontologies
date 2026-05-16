use std::collections::HashSet;
use std::io::Cursor;

use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::sparql::QueryResults;
use oxigraph::store::Store;

use crate::graph::GraphStore;
use crate::state::StateDb;
use std::sync::Arc;

/// JSON field key for the RDF triple count reported in API responses.
pub const TRIPLE_COUNT_KEY: &str = "triple_count";

pub struct OntologyService;

impl OntologyService {
    /// Validate RDF syntax. Returns a JSON report (never errors on bad input).
    ///
    /// # Examples
    ///
    /// Valid minimal Turtle returns `"valid": true`:
    ///
    /// ```
    /// # use open_ontologies::ontology::OntologyService;
    /// let ttl = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<urn:ex:A> a owl:Class .";
    /// let json = OntologyService::validate_string(ttl).unwrap();
    /// let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    /// assert_eq!(v["valid"], true);
    /// assert!(v["triple_count"].as_u64().unwrap() > 0);
    /// ```
    ///
    /// Malformed TTL returns `"valid": false` (no panic, no `Err`):
    ///
    /// ```
    /// # use open_ontologies::ontology::OntologyService;
    /// let bad = "@prefix owl: <http://www.w3.org/2002/07/owl#>";   // missing closing dot
    /// let json = OntologyService::validate_string(bad).unwrap();
    /// let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    /// assert_eq!(v["valid"], false);
    /// assert!(!v["errors"].as_array().unwrap().is_empty());
    /// ```
    pub fn validate_string(content: &str) -> anyhow::Result<String> {
        match GraphStore::validate_turtle(content) {
            Ok(count) => Ok(serde_json::json!({
                "valid": true,
                TRIPLE_COUNT_KEY: count,
                "errors": []
            })
            .to_string()),
            Err(e) => Ok(serde_json::json!({
                "valid": false,
                TRIPLE_COUNT_KEY: 0,
                "errors": [e.to_string()]
            })
            .to_string()),
        }
    }

    /// Validate an RDF file.
    pub fn validate_file(path: &str) -> anyhow::Result<String> {
        match GraphStore::validate_file(path) {
            Ok(count) => Ok(serde_json::json!({
                "valid": true,
                "path": path,
                TRIPLE_COUNT_KEY: count,
                "errors": []
            })
            .to_string()),
            Err(e) => Ok(serde_json::json!({
                "valid": false,
                "path": path,
                TRIPLE_COUNT_KEY: 0,
                "errors": [e.to_string()]
            })
            .to_string()),
        }
    }

    /// Convert between RDF formats.
    ///
    /// # Examples
    ///
    /// Convert Turtle to N-Triples — output contains the expected IRI:
    ///
    /// ```
    /// # use open_ontologies::ontology::OntologyService;
    /// let ttl = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<urn:ex:A> a owl:Class .";
    /// let nt = OntologyService::convert(ttl, "turtle", "ntriples").unwrap();
    /// assert!(nt.contains("<urn:ex:A>"));
    /// ```
    pub fn convert(content: &str, _from: &str, to: &str) -> anyhow::Result<String> {
        let store = GraphStore::new();
        store.load_turtle(content, None)?;
        store.serialize(to)
    }

    /// Diff two ontologies. Returns added/removed triples.
    ///
    /// # Examples
    ///
    /// Identical content produces zero added and zero removed:
    ///
    /// ```
    /// # use open_ontologies::ontology::OntologyService;
    /// let ttl = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<urn:ex:A> a owl:Class .";
    /// let json = OntologyService::diff(ttl, ttl).unwrap();
    /// let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    /// assert_eq!(v["added"], 0);
    /// assert_eq!(v["removed"], 0);
    /// ```
    ///
    /// Adding a class to the new content produces a non-zero `added` count:
    ///
    /// ```
    /// # use open_ontologies::ontology::OntologyService;
    /// let old = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<urn:ex:A> a owl:Class .";
    /// let new = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<urn:ex:A> a owl:Class .\n<urn:ex:B> a owl:Class .";
    /// let json = OntologyService::diff(old, new).unwrap();
    /// let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    /// assert!(v["added"].as_u64().unwrap() > 0);
    /// assert_eq!(v["removed"], 0);
    /// ```
    pub fn diff(old_content: &str, new_content: &str) -> anyhow::Result<String> {
        let old_store = Store::new()?;
        let new_store = Store::new()?;

        let old_reader = Cursor::new(old_content.as_bytes());
        for quad in RdfParser::from_format(RdfFormat::Turtle).for_reader(old_reader) {
            old_store.insert(&quad?)?;
        }

        let new_reader = Cursor::new(new_content.as_bytes());
        for quad in RdfParser::from_format(RdfFormat::Turtle).for_reader(new_reader) {
            new_store.insert(&quad?)?;
        }

        let old_triples: HashSet<String> = old_store
            .iter()
            .filter_map(|q| q.ok())
            .map(|q| format!("{} {} {}", q.subject, q.predicate, q.object))
            .collect();

        let new_triples: HashSet<String> = new_store
            .iter()
            .filter_map(|q| q.ok())
            .map(|q| format!("{} {} {}", q.subject, q.predicate, q.object))
            .collect();

        let added: Vec<&String> = new_triples.difference(&old_triples).collect();
        let removed: Vec<&String> = old_triples.difference(&new_triples).collect();

        Ok(serde_json::json!({
            "added": added.len(),
            "removed": removed.len(),
            "added_triples": added,
            "removed_triples": removed,
        })
        .to_string())
    }

    /// Collect raw lint issues from a Store.
    fn collect_lint_issues(store: &Store) -> anyhow::Result<Vec<serde_json::Value>> {
        let mut issues: Vec<serde_json::Value> = Vec::new();

        // Find classes without rdfs:label
        let query = r#"
            SELECT ?class WHERE {
                { ?class a <http://www.w3.org/2002/07/owl#Class> }
                UNION
                { ?class a <http://www.w3.org/2000/01/rdf-schema#Class> }
                FILTER NOT EXISTS { ?class <http://www.w3.org/2000/01/rdf-schema#label> ?label }
            }
        "#;
        if let Ok(QueryResults::Solutions(solutions)) = store.query(query) {
            for row in solutions.flatten() {
                if let Some(term) = row.get("class") {
                    issues.push(serde_json::json!({
                        "severity": "warning",
                        "type": "missing_label",
                        "entity": term.to_string(),
                        "message": format!("{} has no rdfs:label", term),
                    }));
                }
            }
        }

        // Find classes without rdfs:comment
        let query = r#"
            SELECT ?class WHERE {
                { ?class a <http://www.w3.org/2002/07/owl#Class> }
                UNION
                { ?class a <http://www.w3.org/2000/01/rdf-schema#Class> }
                FILTER NOT EXISTS { ?class <http://www.w3.org/2000/01/rdf-schema#comment> ?comment }
            }
        "#;
        if let Ok(QueryResults::Solutions(solutions)) = store.query(query) {
            for row in solutions.flatten() {
                if let Some(term) = row.get("class") {
                    issues.push(serde_json::json!({
                        "severity": "warning",
                        "type": "missing_comment",
                        "entity": term.to_string(),
                        "message": format!("{} has no rdfs:comment", term),
                    }));
                }
            }
        }

        // Find properties without domain
        let query = r#"
            SELECT ?prop WHERE {
                { ?prop a <http://www.w3.org/2002/07/owl#ObjectProperty> }
                UNION
                { ?prop a <http://www.w3.org/2002/07/owl#DatatypeProperty> }
                FILTER NOT EXISTS { ?prop <http://www.w3.org/2000/01/rdf-schema#domain> ?d }
            }
        "#;
        if let Ok(QueryResults::Solutions(solutions)) = store.query(query) {
            for row in solutions.flatten() {
                if let Some(term) = row.get("prop") {
                    issues.push(serde_json::json!({
                        "severity": "info",
                        "type": "missing_domain",
                        "entity": term.to_string(),
                        "message": format!("{} has no rdfs:domain", term),
                    }));
                }
            }
        }

        Ok(issues)
    }

    /// Lint an ontology with feedback-based suppression.
    ///
    /// Pass `None` for `db` to skip feedback suppression (all issues are reported as-is).
    ///
    /// # Examples
    ///
    /// A class without `rdfs:label` is flagged even when `db` is `None`:
    ///
    /// ```
    /// # use open_ontologies::ontology::OntologyService;
    /// let ttl = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<urn:ex:A> a owl:Class .";
    /// let json = OntologyService::lint_with_feedback(ttl, None).unwrap();
    /// let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    /// // At least one issue (missing_label) is reported
    /// assert!(v["issue_count"].as_u64().unwrap() > 0);
    /// assert_eq!(v["suppressed_count"], 0);
    /// ```
    pub fn lint_with_feedback(content: &str, db: Option<&crate::state::StateDb>) -> anyhow::Result<String> {
        let store = Store::new()?;
        let reader = Cursor::new(content.as_bytes());
        for quad in RdfParser::from_format(RdfFormat::Turtle).for_reader(reader) {
            store.insert(&quad?)?;
        }

        let raw_issues = Self::collect_lint_issues(&store)?;
        let mut issues: Vec<serde_json::Value> = Vec::new();
        let mut suppressed_count: u64 = 0;

        for issue in raw_issues {
            if let Some(db) = db {
                let rule_id = issue["type"].as_str().unwrap_or("");
                let entity = issue["entity"].as_str().unwrap_or("");
                match crate::feedback::get_feedback_adjustment(db, "lint", rule_id, entity) {
                    crate::feedback::FeedbackAction::Suppress => {
                        suppressed_count += 1;
                        continue;
                    }
                    crate::feedback::FeedbackAction::Downgrade => {
                        let original = issue["severity"].as_str().unwrap_or("info");
                        let downgraded = crate::feedback::downgrade_severity(original);
                        let mut adjusted = issue.clone();
                        adjusted["original_severity"] = serde_json::json!(original);
                        adjusted["adjusted_severity"] = serde_json::json!(downgraded);
                        adjusted["severity"] = serde_json::json!(downgraded);
                        issues.push(adjusted);
                    }
                    crate::feedback::FeedbackAction::Keep => {
                        issues.push(issue);
                    }
                }
            } else {
                issues.push(issue);
            }
        }

        Ok(serde_json::json!({
            "issues": issues,
            "issue_count": issues.len(),
            "suppressed_count": suppressed_count,
        })
        .to_string())
    }

    /// Lint an ontology -- check for missing labels, comments, domains.
    ///
    /// # Examples
    ///
    /// A class missing `rdfs:label` triggers a `missing_label` warning:
    ///
    /// ```
    /// # use open_ontologies::ontology::OntologyService;
    /// let ttl = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<urn:ex:A> a owl:Class .";
    /// let json = OntologyService::lint(ttl).unwrap();
    /// let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    /// assert!(v["issue_count"].as_u64().unwrap() > 0);
    /// let issues = v["issues"].as_array().unwrap();
    /// let has_missing_label = issues.iter().any(|i| i["type"] == "missing_label");
    /// assert!(has_missing_label);
    /// ```
    pub fn lint(content: &str) -> anyhow::Result<String> {
        Self::lint_with_feedback(content, None)
    }

    /// Save a named version (snapshot) of the current graph store.
    ///
    /// Requires a live [`StateDb`] and [`GraphStore`]. Use `StateDb::open(Path::new(":memory:"))`
    /// for in-memory testing.
    ///
    /// # Examples
    ///
    /// ```
    /// # use open_ontologies::ontology::OntologyService;
    /// # use open_ontologies::state::StateDb;
    /// # use open_ontologies::graph::GraphStore;
    /// # use std::sync::Arc;
    /// # use std::path::Path;
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = Arc::new(GraphStore::new());
    /// store.load_turtle(
    ///     "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<urn:ex:A> a owl:Class .",
    ///     None,
    /// ).unwrap();
    /// let json = OntologyService::save_version(&db, &store, "v1").unwrap();
    /// let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    /// assert_eq!(v["ok"], true);
    /// assert_eq!(v["label"], "v1");
    /// assert!(v["triple_count"].as_u64().unwrap() >= 1);
    /// ```
    ///
    /// Multiple saves accumulate independently — each label is stored:
    ///
    /// ```
    /// # use open_ontologies::ontology::OntologyService;
    /// # use open_ontologies::state::StateDb;
    /// # use open_ontologies::graph::GraphStore;
    /// # use std::sync::Arc;
    /// # use std::path::Path;
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = Arc::new(GraphStore::new());
    /// store.load_turtle(
    ///     "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<urn:ex:C> a owl:Class .",
    ///     None,
    /// ).unwrap();
    /// OntologyService::save_version(&db, &store, "alpha").unwrap();
    /// OntologyService::save_version(&db, &store, "beta").unwrap();
    /// let json = OntologyService::list_versions(&db).unwrap();
    /// let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    /// assert_eq!(v["versions"].as_array().unwrap().len(), 2);
    /// ```
    pub fn save_version(db: &StateDb, store: &Arc<GraphStore>, label: &str) -> anyhow::Result<String> {
        let content = store.snapshot("ntriples")?;
        let count = store.triple_count();
        let conn = db.conn();
        conn.execute(
            "INSERT INTO ontology_versions (label, triple_count, content, format) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![label, count as i64, content, "ntriples"],
        )?;
        Ok(serde_json::json!({
            "ok": true,
            "label": label,
            TRIPLE_COUNT_KEY: count,
        }).to_string())
    }

    /// List all saved ontology versions.
    ///
    /// Returns a JSON object with a `"versions"` array (empty when no snapshots have been saved).
    ///
    /// # Examples
    ///
    /// Fresh database has an empty versions list:
    ///
    /// ```
    /// # use open_ontologies::ontology::OntologyService;
    /// # use open_ontologies::state::StateDb;
    /// # use std::path::Path;
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let json = OntologyService::list_versions(&db).unwrap();
    /// let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    /// assert!(v["versions"].is_array());
    /// assert_eq!(v["versions"].as_array().unwrap().len(), 0);
    /// ```
    ///
    /// After saving a version, `list_versions` returns it:
    ///
    /// ```
    /// # use open_ontologies::ontology::OntologyService;
    /// # use open_ontologies::state::StateDb;
    /// # use open_ontologies::graph::GraphStore;
    /// # use std::sync::Arc;
    /// # use std::path::Path;
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = Arc::new(GraphStore::new());
    /// store.load_turtle(
    ///     "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<urn:ex:B> a owl:Class .",
    ///     None,
    /// ).unwrap();
    /// OntologyService::save_version(&db, &store, "snap-a").unwrap();
    /// let json = OntologyService::list_versions(&db).unwrap();
    /// let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    /// let versions = v["versions"].as_array().unwrap();
    /// assert_eq!(versions.len(), 1);
    /// assert_eq!(versions[0]["label"], "snap-a");
    /// ```
    pub fn list_versions(db: &StateDb) -> anyhow::Result<String> {
        let conn = db.conn();
        let mut stmt = conn.prepare(
            "SELECT id, label, triple_count, format, created_at FROM ontology_versions ORDER BY id DESC"
        )?;
        let versions: Vec<serde_json::Value> = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "label": row.get::<_, String>(1)?,
                TRIPLE_COUNT_KEY: row.get::<_, i64>(2)?,
                "format": row.get::<_, String>(3)?,
                "created_at": row.get::<_, String>(4)?,
            }))
        })?.filter_map(|r| r.ok()).collect();
        Ok(serde_json::json!({"versions": versions}).to_string())
    }

    /// Rollback the graph store to a previously saved version.
    ///
    /// Restores the store contents from the snapshot identified by `label` and returns a JSON
    /// object reporting the number of triples restored.
    ///
    /// # Examples
    ///
    /// Save, clear, rollback — the store is restored:
    ///
    /// ```
    /// # use open_ontologies::ontology::OntologyService;
    /// # use open_ontologies::state::StateDb;
    /// # use open_ontologies::graph::GraphStore;
    /// # use std::sync::Arc;
    /// # use std::path::Path;
    /// let db = StateDb::open(Path::new(":memory:")).unwrap();
    /// let store = Arc::new(GraphStore::new());
    /// store.load_turtle(
    ///     "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<urn:ex:A> a owl:Class .",
    ///     None,
    /// ).unwrap();
    /// OntologyService::save_version(&db, &store, "snap1").unwrap();
    /// store.clear().unwrap();
    /// assert_eq!(store.triple_count(), 0);
    /// let json = OntologyService::rollback_version(&db, &store, "snap1").unwrap();
    /// let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    /// assert_eq!(v["ok"], true);
    /// assert_eq!(v["label"], "snap1");
    /// assert!(v["triples_restored"].as_u64().unwrap() >= 1);
    /// assert!(store.triple_count() >= 1);
    /// ```
    pub fn rollback_version(db: &StateDb, store: &Arc<GraphStore>, label: &str) -> anyhow::Result<String> {
        let conn = db.conn();
        let content: String = conn.query_row(
            "SELECT content FROM ontology_versions WHERE label = ?1 ORDER BY id DESC LIMIT 1",
            rusqlite::params![label],
            |row| row.get(0),
        )?;
        store.clear()?;
        let count = store.load_ntriples(&content)?;
        Ok(serde_json::json!({
            "ok": true,
            "label": label,
            "triples_restored": count,
        }).to_string())
    }
}
