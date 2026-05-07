//! Ontology Commands — core RDF/OWL operations

use clap_noun_verb::Result as NounVerbResult;
use clap_noun_verb_macros::verb;
use serde::Serialize;

use super::helpers::{DEFAULT_DATA_DIR, setup, to_verb_err};
use open_ontologies::graph::GraphStore;
use open_ontologies::ontology::OntologyService;
use open_ontologies::reason::Reasoner;
use open_ontologies::shacl::ShaclValidator;

// ── output types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ValidateOutput {
    pub ok: bool,
    pub triples: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct LoadOutput {
    pub ok: bool,
    pub triples_loaded: usize,
    pub path: String,
}

#[derive(Serialize)]
pub struct SaveOutput {
    pub ok: bool,
    pub path: String,
    pub format: String,
}

#[derive(Serialize)]
pub struct ClearOutput {
    pub ok: bool,
    pub message: String,
}

#[derive(Serialize)]
pub struct StatusOutput {
    pub status: String,
    pub version: String,
    pub triples_loaded: usize,
}

// ── domain helpers (not #[verb]) ─────────────────────────────────────────

fn do_validate(input: &str) -> ValidateOutput {
    let result = if input == "-" {
        let mut buf = String::new();
        match std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf) {
            Ok(_) => GraphStore::validate_turtle(&buf),
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    } else {
        GraphStore::validate_file(input)
    };
    match result {
        Ok(count) => ValidateOutput { ok: true, triples: count, error: None },
        Err(e) => ValidateOutput { ok: false, triples: 0, error: Some(e.to_string()) },
    }
}

fn do_lint(input: &str, data_dir: &str) -> NounVerbResult<serde_json::Value> {
    let (db, _graph) = setup(data_dir).map_err(to_verb_err)?;
    let content = if input == "-" {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf).map_err(to_verb_err)?;
        buf
    } else {
        std::fs::read_to_string(input).map_err(to_verb_err)?
    };
    let result = OntologyService::lint_with_feedback(&content, Some(&db))
        .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}

fn do_convert(path: &str, to: &str, output: Option<&str>) -> NounVerbResult<serde_json::Value> {
    let store = GraphStore::new();
    store.load_file(path).map_err(to_verb_err)?;
    let content = store.serialize(to).map_err(to_verb_err)?;
    if let Some(out_path) = output {
        std::fs::write(out_path, &content).map_err(to_verb_err)?;
        Ok(serde_json::json!({"ok": true, "path": out_path, "format": to}))
    } else {
        println!("{}", content);
        Ok(serde_json::json!({"ok": true, "path": null, "format": to}))
    }
}

fn do_sparql_query(query_str: &str, data_dir: &str) -> NounVerbResult<serde_json::Value> {
    let (_db, graph) = setup(data_dir).map_err(to_verb_err)?;
    let q = if query_str == "-" {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf).map_err(to_verb_err)?;
        buf
    } else {
        query_str.to_string()
    };
    let result = graph.sparql_select(&q).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}

// ── verbs ─────────────────────────────────────────────────────────────────

/// Validate RDF/OWL syntax (file or stdin with -)
#[verb]
fn validate(input: String) -> NounVerbResult<ValidateOutput> {
    Ok(do_validate(&input))
}

/// Load RDF file into in-memory graph store
#[verb]
fn load(path: String, data_dir: Option<String>) -> NounVerbResult<LoadOutput> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    let (_db, graph) = setup(dir).map_err(to_verb_err)?;
    let count = graph.load_file(&path).map_err(to_verb_err)?;
    Ok(LoadOutput { ok: true, triples_loaded: count, path })
}

/// Save ontology to file
#[verb]
fn save(path: String, format: Option<String>, data_dir: Option<String>) -> NounVerbResult<SaveOutput> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    let (_db, graph) = setup(dir).map_err(to_verb_err)?;
    let fmt = format.unwrap_or_else(|| "turtle".to_string());
    graph.save_file(&path, &fmt).map_err(to_verb_err)?;
    Ok(SaveOutput { ok: true, path, format: fmt })
}

/// Clear in-memory store
#[verb]
fn clear(data_dir: Option<String>) -> NounVerbResult<ClearOutput> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    let (_db, graph) = setup(dir).map_err(to_verb_err)?;
    graph.clear().map_err(to_verb_err)?;
    Ok(ClearOutput { ok: true, message: "Store cleared".to_string() })
}

/// Show triple count, classes, properties, individuals
#[verb]
fn stats(data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    let (_db, graph) = setup(dir).map_err(to_verb_err)?;
    let s = graph.get_stats().unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    serde_json::from_str::<serde_json::Value>(&s).map_err(to_verb_err)
}

/// Run SPARQL query (or stdin with -)
#[verb]
fn sparql(sparql_query: String, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    do_sparql_query(&sparql_query, dir)
}

/// Compare two ontology files
#[verb]
fn diff(old_path: String, new_path: String) -> NounVerbResult<serde_json::Value> {
    let old = std::fs::read_to_string(&old_path).map_err(to_verb_err)?;
    let new = std::fs::read_to_string(&new_path).map_err(to_verb_err)?;
    let result = OntologyService::diff(&old, &new).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}

/// Lint: check for missing labels, domains, ranges
#[verb]
fn lint(input: String, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    do_lint(&input, dir)
}

/// Convert between RDF formats
#[verb]
fn convert(path: String, to: String, output: Option<String>) -> NounVerbResult<serde_json::Value> {
    do_convert(&path, &to, output.as_deref())
}

/// Server health and loaded triple count
#[verb]
fn status(data_dir: Option<String>) -> NounVerbResult<StatusOutput> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    let (_db, graph) = setup(dir).map_err(to_verb_err)?;
    Ok(StatusOutput {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        triples_loaded: graph.triple_count(),
    })
}

/// Save a named version snapshot
#[verb]
fn version(label: String, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    let (db, graph) = setup(dir).map_err(to_verb_err)?;
    let result = OntologyService::save_version(&db, &graph, &label)
        .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}

/// List saved version snapshots
#[verb]
fn history(data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    let (db, _graph) = setup(dir).map_err(to_verb_err)?;
    let result = OntologyService::list_versions(&db)
        .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}

/// Restore a previous version
#[verb]
fn rollback(label: String, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    let (db, graph) = setup(dir).map_err(to_verb_err)?;
    let result = OntologyService::rollback_version(&db, &graph, &label)
        .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}

/// Run inference (rdfs, owl-rl, owl-rl-ext, owl-dl)
#[verb]
fn reason(profile: Option<String>, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    let (_db, graph) = setup(dir).map_err(to_verb_err)?;
    let p = profile.unwrap_or_else(|| "rdfs".to_string());
    let result = Reasoner::run(&graph, &p, true)
        .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e));
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}

/// Validate against SHACL shapes
#[verb]
fn shacl(shapes: String, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    let (_db, graph) = setup(dir).map_err(to_verb_err)?;
    let shapes_content = std::fs::read_to_string(&shapes).map_err(to_verb_err)?;
    let result = ShaclValidator::validate(&graph, &shapes_content)
        .unwrap_err_as_json();
    serde_json::from_str::<serde_json::Value>(&result).map_err(to_verb_err)
}

// helper trait to unify Ok/Err -> JSON string
trait UnwrapErrAsJson {
    fn unwrap_err_as_json(self) -> String;
}

impl UnwrapErrAsJson for Result<String, anyhow::Error> {
    fn unwrap_err_as_json(self) -> String {
        self.unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }
}
