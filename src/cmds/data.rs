//! Data Commands — ingestion, mapping, remote pull/push, schema import
//!
//! Provides CLI verbs for pulling ontologies from URLs, ingesting structured
//! data (CSV, JSON, Parquet, …) into RDF, generating mapping configs, running
//! SQL queries against PostgreSQL/DuckDB, and resolving `owl:imports` chains.
//!
//! # CLI usage
//!
//! ```no_run
//! // Pull an ontology from a remote HTTP URL into the local store:
//! //   open-ontologies data pull https://example.org/onto.ttl
//! //
//! // Ingest a CSV file using an auto-generated mapping:
//! //   open-ontologies data ingest path/to/data.csv
//! //
//! // Ingest a CSV with an explicit JSON mapping and custom base IRI:
//! //   open-ontologies data ingest path/to/data.csv \
//! //     --mapping mapping.json \
//! //     --base-iri http://myorg.example/data/
//! //
//! // Run the full pipeline (ingest → SHACL validate → OWL-RL reason):
//! //   open-ontologies data extend path/to/data.csv \
//! //     --shapes shapes.ttl --profile owl-rl
//! ```

use clap_noun_verb::Result as NounVerbResult;
use clap_noun_verb_macros::verb;
use serde::Serialize;

use super::helpers::{DEFAULT_DATA_DIR, setup, to_verb_err};
use open_ontologies::graph::GraphStore;
use open_ontologies::ingest::DataIngester;
use open_ontologies::mapping::MappingConfig;

// ── output types ─────────────────────────────────────────────────────────

/// Result returned after `data pull` successfully loads an ontology from a URL.
///
/// `triples_loaded` is the count of triples added to the active store;
/// `source` echoes the URL that was fetched.
///
/// # Examples
///
/// ```no_run
/// // CLI: open-ontologies data pull https://www.w3.org/2004/02/skos/core
/// // Returns PullOutput {
/// //   ok: true,
/// //   triples_loaded: 252,
/// //   source: "https://www.w3.org/2004/02/skos/core",
/// // }
/// ```
#[derive(Serialize)]
pub struct PullOutput {
    pub ok: bool,
    pub triples_loaded: usize,
    pub source: String,
}

/// Result returned after `data import-owl` resolves an `owl:imports` chain.
///
/// `imported` is the number of unique URLs that were fetched and loaded
/// (duplicates are skipped); `urls` lists those URLs in resolution order.
/// The chain is walked up to `max_depth` hops (default 10).
///
/// # Examples
///
/// ```no_run
/// // CLI: open-ontologies data import-owl --max-depth 3
/// //
/// // When two transitive imports are resolved:
/// // Returns ImportOwlOutput {
/// //   ok: true,
/// //   imported: 2,
/// //   urls: ["https://example.org/a.ttl", "https://example.org/b.ttl"],
/// // }
/// //
/// // When the store has no owl:imports triples:
/// // Returns ImportOwlOutput { ok: true, imported: 0, urls: [] }
/// ```
#[derive(Serialize)]
pub struct ImportOwlOutput {
    pub ok: bool,
    pub imported: usize,
    pub urls: Vec<String>,
}

// ── domain helpers ────────────────────────────────────────────────────────

fn resolve_mapping(mapping: Option<&str>, rows: &[std::collections::HashMap<String, String>], base: &str) -> NounVerbResult<MappingConfig> {
    if let Some(mapping_path) = mapping {
        let content = std::fs::read_to_string(mapping_path).map_err(to_verb_err)?;
        serde_json::from_str::<MappingConfig>(&content).map_err(to_verb_err)
    } else {
        let headers = DataIngester::extract_headers(rows);
        Ok(MappingConfig::from_headers(&headers, base, &format!("{}Thing", base)))
    }
}

fn do_ingest(path: &str, format: Option<&str>, mapping: Option<&str>, base: &str, data_dir: &str) -> NounVerbResult<serde_json::Value> {
    let (_db, graph) = setup(data_dir).map_err(to_verb_err)?;
    let rows = DataIngester::parse_file_with_format(path, format).map_err(to_verb_err)?;
    if rows.is_empty() {
        return Ok(serde_json::json!({"ok": true, "triples_loaded": 0, "rows": 0}));
    }
    let cfg = resolve_mapping(mapping, &rows, base)?;
    let ntriples = cfg.rows_to_ntriples(&rows);
    let count = graph.load_ntriples(&ntriples).map_err(to_verb_err)?;
    Ok(serde_json::json!({"ok": true, "triples_loaded": count, "rows": rows.len()}))
}

fn do_map(data_path: &str, format: Option<&str>, save: Option<&str>, data_dir: &str) -> NounVerbResult<serde_json::Value> {
    let (_db, graph) = setup(data_dir).map_err(to_verb_err)?;
    let rows = DataIngester::parse_file_with_format(data_path, format).map_err(to_verb_err)?;
    let headers = DataIngester::extract_headers(&rows);
    let classes_q = r#"SELECT DISTINCT ?c WHERE { { ?c a <http://www.w3.org/2002/07/owl#Class> } UNION { ?c a <http://www.w3.org/2000/01/rdf-schema#Class> } }"#;
    let props_q = r#"SELECT DISTINCT ?p WHERE { { ?p a <http://www.w3.org/2002/07/owl#ObjectProperty> } UNION { ?p a <http://www.w3.org/2002/07/owl#DatatypeProperty> } UNION { ?p a <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property> } }"#;
    let classes = graph.sparql_select(classes_q).unwrap_or_default();
    let props = graph.sparql_select(props_q).unwrap_or_default();
    let mapping = MappingConfig::from_headers(&headers, "http://example.org/data/", "http://example.org/data/Thing");
    let mapping_json = serde_json::to_string_pretty(&mapping).unwrap_or_default();
    if let Some(save_path) = save {
        std::fs::write(save_path, &mapping_json).map_err(to_verb_err)?;
        return Ok(serde_json::json!({"ok": true, "saved": save_path}));
    }
    let extract = |json: &str, var: &str| -> Vec<String> {
        serde_json::from_str::<serde_json::Value>(json).ok()
            .and_then(|v| v["results"].as_array().cloned()).unwrap_or_default()
            .iter().filter_map(|r| r[var].as_str().map(|s| s.trim_matches(|c| c == '<' || c == '>').to_string()))
            .collect()
    };
    Ok(serde_json::json!({
        "data_fields": headers,
        "ontology_classes": extract(&classes, "c"),
        "ontology_properties": extract(&props, "p"),
        "suggested_mapping": serde_json::from_str::<serde_json::Value>(&mapping_json).unwrap_or_default(),
    }))
}

fn do_pull_fetch(url: &str, sparql: bool, query: Option<&str>) -> NounVerbResult<String> {
    let content = tokio::runtime::Handle::current().block_on(async {
        if sparql {
            let q = query.unwrap_or("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }");
            GraphStore::fetch_sparql(url, q).await
        } else {
            GraphStore::fetch_url(url).await
        }
    }).map_err(to_verb_err)?;
    Ok(content)
}

fn do_import_owl(max_depth: usize, data_dir: &str) -> NounVerbResult<ImportOwlOutput> {
    let (_db, graph) = setup(data_dir).map_err(to_verb_err)?;
    let mut imported: Vec<String> = Vec::new();
    let q = "SELECT ?import WHERE { ?onto <http://www.w3.org/2002/07/owl#imports> ?import }";
    let mut to_import = collect_imports(&graph, q);
    let mut depth = 0;
    while !to_import.is_empty() && depth < max_depth {
        let batch = std::mem::take(&mut to_import);
        for url in batch {
            if imported.contains(&url) { continue; }
            let result = tokio::runtime::Handle::current().block_on(GraphStore::fetch_url(&url));
            if let Ok(content) = result
                && let Ok(count) = graph.load_turtle(&content, None) {
                    eprintln!("Imported {} ({} triples)", url, count);
                    imported.push(url);
                }
        }
        depth += 1;
    }
    Ok(ImportOwlOutput { ok: true, imported: imported.len(), urls: imported })
}

fn collect_imports(graph: &open_ontologies::graph::GraphStore, q: &str) -> Vec<String> {
    let mut result = Vec::new();
    if let Ok(json) = graph.sparql_select(q)
        && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json)
            && let Some(rows) = parsed["results"].as_array() {
                for row in rows {
                    if let Some(uri) = row["import"].as_str() {
                        result.push(uri.trim_matches(|c| c == '<' || c == '>').to_string());
                    }
                }
            }
    result
}

fn do_sql_ingest(connection: &str, sql: &str, mapping: Option<&str>, inline_mapping: bool, base_iri: &str, data_dir: &str) -> NounVerbResult<serde_json::Value> {
    let (_db, graph) = setup(data_dir).map_err(to_verb_err)?;
    let driver = open_ontologies::sqlsource::detect_driver(connection).map_err(to_verb_err)?;
    let rows = tokio::runtime::Handle::current()
        .block_on(open_ontologies::sqlsource::query_rows(connection, sql))
        .map_err(to_verb_err)?;
    if rows.is_empty() {
        return Ok(serde_json::json!({"ok": true, "driver": driver.as_str(), "triples_loaded": 0, "rows_processed": 0}));
    }
    let cfg = if let Some(m) = mapping {
        let content = if inline_mapping { m.to_string() } else { std::fs::read_to_string(m).map_err(to_verb_err)? };
        serde_json::from_str::<MappingConfig>(&content).map_err(to_verb_err)?
    } else {
        let headers = DataIngester::extract_headers(&rows);
        MappingConfig::from_headers(&headers, base_iri, &format!("{}Thing", base_iri))
    };
    let ntriples = cfg.rows_to_ntriples(&rows);
    let count = graph.load_ntriples(&ntriples).map_err(to_verb_err)?;
    Ok(serde_json::json!({"ok": true, "driver": driver.as_str(), "triples_loaded": count, "rows_processed": rows.len(), "mapping_fields": cfg.mappings.len()}))
}

fn do_extend(data_path: &str, format: Option<&str>, mapping: Option<&str>, shapes: Option<&str>, profile: Option<&str>, data_dir: &str) -> NounVerbResult<serde_json::Value> {
    use open_ontologies::shacl::ShaclValidator;
    use open_ontologies::reason::Reasoner;
    let (_db, graph) = setup(data_dir).map_err(to_verb_err)?;
    let base = "http://example.org/data/";
    let rows = DataIngester::parse_file_with_format(data_path, format).map_err(to_verb_err)?;
    let cfg = resolve_mapping(mapping, &rows, base)?;
    let ntriples = cfg.rows_to_ntriples(&rows);
    let triples_loaded = graph.load_ntriples(&ntriples).map_err(to_verb_err)?;
    let shacl_result = shapes.and_then(|sp| {
        std::fs::read_to_string(sp).ok()
            .and_then(|sc| ShaclValidator::validate(&graph, &sc).ok())
            .and_then(|r| serde_json::from_str::<serde_json::Value>(&r).ok())
    });
    let reason_result = profile.and_then(|p| Reasoner::run(&graph, p, true).ok()
        .and_then(|r| serde_json::from_str::<serde_json::Value>(&r).ok()));
    Ok(serde_json::json!({"ok": true, "triples_loaded": triples_loaded, "rows": rows.len(), "shacl": shacl_result, "reason": reason_result}))
}

// ── verbs ─────────────────────────────────────────────────────────────────

/// Ingest structured data into RDF
#[verb]
fn ingest(path: String, format: Option<String>, mapping: Option<String>, base_iri: Option<String>, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let base = base_iri.as_deref().unwrap_or("http://example.org/data/");
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    do_ingest(&path, format.as_deref(), mapping.as_deref(), base, dir)
}

/// Generate mapping config from data file + ontology
#[verb]
fn map(data_path: String, format: Option<String>, save: Option<String>, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    do_map(&data_path, format.as_deref(), save.as_deref(), dir)
}

/// Fetch ontology from URL or SPARQL endpoint
#[verb]
fn pull(url: String, sparql: Option<bool>, sparql_query: Option<String>, data_dir: Option<String>) -> NounVerbResult<PullOutput> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    let (_db, graph) = setup(dir).map_err(to_verb_err)?;
    let content = do_pull_fetch(&url, sparql.unwrap_or(false), sparql_query.as_deref())?;
    let count = graph.load_turtle(&content, None).map_err(to_verb_err)?;
    Ok(PullOutput { ok: true, triples_loaded: count, source: url })
}

/// Push ontology to SPARQL endpoint
#[verb]
fn push(endpoint: String, graph_name: Option<String>, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    let (_db, g) = setup(dir).map_err(to_verb_err)?;
    let content = g.serialize("ntriples").map_err(to_verb_err)?;
    // Sync verb body invoked from within `#[tokio::main]` — must yield the
    // current worker thread before driving an async future, otherwise tokio
    // panics with "Cannot start a runtime from within a runtime".
    let msg = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(GraphStore::push_sparql_graph(
            &endpoint,
            &content,
            graph_name.as_deref(),
            &[],
        ))
    })
    .map_err(to_verb_err)?;
    Ok(serde_json::json!({"ok": true, "message": msg, "graph": graph_name}))
}

/// Resolve and load owl:imports chain
#[verb]
fn import_owl(max_depth: Option<usize>, data_dir: Option<String>) -> NounVerbResult<ImportOwlOutput> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    do_import_owl(max_depth.unwrap_or(10), dir)
}

/// Import database schema as OWL ontology (Postgres or DuckDB)
#[verb]
fn import_schema(connection: String, base_iri: Option<String>, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    let base = base_iri.unwrap_or_else(|| "http://example.org/db/".to_string());
    do_import_schema_inner(&connection, &base, dir)
}

fn do_import_schema_inner(connection: &str, base_iri: &str, data_dir: &str) -> NounVerbResult<serde_json::Value> {
    let (_db, graph) = setup(data_dir).map_err(to_verb_err)?;
    let driver = open_ontologies::sqlsource::detect_driver(connection).map_err(to_verb_err)?;
    let tables = fetch_schema_tables(connection, &driver)?;
    let turtle = open_ontologies::schema::SchemaIntrospector::generate_turtle(&tables, base_iri);
    GraphStore::validate_turtle(&turtle).map_err(to_verb_err)?;
    let count = graph.load_turtle(&turtle, Some(base_iri)).map_err(to_verb_err)?;
    Ok(serde_json::json!({"ok": true, "driver": driver.as_str(), "tables": tables.len(), "triples": count, "base_iri": base_iri}))
}

#[cfg_attr(not(any(feature = "postgres", feature = "duckdb")), allow(unused_variables))]
fn fetch_schema_tables(connection: &str, driver: &open_ontologies::sqlsource::SqlDriver) -> NounVerbResult<Vec<open_ontologies::schema::TableInfo>> {
    match driver {
        open_ontologies::sqlsource::SqlDriver::Postgres => {
            #[cfg(feature = "postgres")]
            {
                tokio::runtime::Handle::current()
                    .block_on(open_ontologies::schema::SchemaIntrospector::introspect_postgres(connection))
                    .map_err(to_verb_err)
            }
            #[cfg(not(feature = "postgres"))]
            {
                Err(clap_noun_verb::NounVerbError::execution_error("postgres feature required"))
            }
        }
        open_ontologies::sqlsource::SqlDriver::DuckDb => {
            #[cfg(feature = "duckdb")]
            {
                let target = open_ontologies::sqlsource::duckdb_target(connection);
                tokio::runtime::Handle::current()
                    .block_on(tokio::task::spawn_blocking(move || open_ontologies::schema::SchemaIntrospector::introspect_duckdb(&target)))
                    .map_err(to_verb_err)?.map_err(to_verb_err)
            }
            #[cfg(not(feature = "duckdb"))]
            {
                Err(clap_noun_verb::NounVerbError::execution_error("duckdb feature required"))
            }
        }
    }
}

/// Run a SQL query and ingest result rows into RDF.
///
/// `connection` is a connection string:
/// - PostgreSQL: `postgres://user:pass@host/db`
/// - DuckDB in-memory: `:memory:` or `duckdb:///:memory:`
/// - DuckDB file: `duckdb:///path/to/file.duckdb`
///
/// Pass `sql = "-"` to read the query from stdin.
/// Set `inline_mapping = true` to interpret `mapping` as a JSON string
/// rather than a file path.
///
/// # Examples
///
/// ```no_run
/// // CLI: open-ontologies data sql-ingest \
/// //   --connection postgres://readonly@localhost/analytics \
/// //   --sql "SELECT id, name, created_at FROM products LIMIT 100" \
/// //   --base-iri http://myorg.example/products/
/// //
/// // Returns JSON:
/// // { "ok": true, "driver": "postgres", "triples_loaded": 300,
/// //   "rows_processed": 100, "mapping_fields": 3 }
/// ```
#[verb]
fn sql_ingest(connection: String, sql: String, mapping: Option<String>, inline_mapping: Option<bool>, base_iri: Option<String>, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let sql_str = if sql == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).map_err(to_verb_err)?;
        buf
    } else {
        sql
    };
    let base = base_iri.unwrap_or_else(|| "http://example.org/data/".to_string());
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    do_sql_ingest(&connection, &sql_str, mapping.as_deref(), inline_mapping.unwrap_or(false), &base, dir)
}

/// Full pipeline: ingest → SHACL validate → OWL reason in a single call.
///
/// Combines `data ingest`, `onto_shacl`, and `onto_reason` into one verb.
/// `shapes` is an optional path to a SHACL shapes file; `profile` is an
/// optional reasoning profile (`"rdfs"` or `"owl-rl"`).  Both can be omitted
/// to run ingest-only.
///
/// # Examples
///
/// ```no_run
/// // CLI: open-ontologies data extend data.csv \
/// //   --shapes ontology/shapes.ttl \
/// //   --profile rdfs
/// //
/// // Returns JSON:
/// // {
/// //   "ok": true,
/// //   "triples_loaded": 480,
/// //   "rows": 160,
/// //   "shacl": { "conforms": true, "violations": [] },
/// //   "reason": { "inferred": 32 }
/// // }
/// ```
#[verb]
fn extend(data_path: String, format: Option<String>, mapping: Option<String>, shapes: Option<String>, profile: Option<String>, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let dir = data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR);
    do_extend(&data_path, format.as_deref(), mapping.as_deref(), shapes.as_deref(), profile.as_deref(), dir)
}

/// Run a batch of commands from a file or stdin
#[verb]
fn batch(input: Option<String>, bail: Option<bool>, data_dir: Option<String>) -> NounVerbResult<serde_json::Value> {
    let (db, graph) = setup(data_dir.as_deref().unwrap_or(DEFAULT_DATA_DIR)).map_err(to_verb_err)?;
    let src = input.unwrap_or_else(|| "-".to_string());
    let batch_input = if src == "-" {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf).map_err(to_verb_err)?;
        buf
    } else {
        std::fs::read_to_string(&src).map_err(to_verb_err)?
    };
    let runner = open_ontologies::batch::BatchRunner::new(db, graph, false);
    let exit_code = tokio::runtime::Handle::current().block_on(runner.run(&batch_input, bail.unwrap_or(false)));
    Ok(serde_json::json!({"exit_code": exit_code}))
}
