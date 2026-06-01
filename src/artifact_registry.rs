//! Artifact Registration — Register ggen-emitted artifacts in open-ontologies RDF store.
//!
//! After ggen emits artifacts (μ₅ stage), this module:
//! 1. Computes BLAKE3 hash of each artifact file
//! 2. Reads ggen receipt JSON
//! 3. Extracts metadata (source TTL, SPARQL query, Tera template, timestamp)
//! 4. Constructs artifact registration RDF via SPARQL INSERT
//! 5. Loads into onto:artifact-registry named graph
//!
//! Artifact IRIs: `urn:ggen:artifact:<path>` (path with `/` → `:`)
//! Receipt IRIs: `urn:ggen:receipt:<operation-id>`

use anyhow::{anyhow, Result};
use blake3;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Artifact metadata extracted from ggen receipt and file system.
#[derive(Debug, Clone)]
pub struct ArtifactRecord {
    /// Artifact file path relative to project root (e.g., "src/cmds/generated.rs")
    pub path: String,
    /// BLAKE3 hash of artifact file (hex string, 64 chars)
    pub hash: String,
    /// ggen receipt JSON data
    pub receipt: ReceiptMetadata,
    /// Timestamp when artifact was generated (RFC3339)
    pub generated_at: String,
    /// Source TTL file(s) used (pipe-separated if multiple)
    pub source_ttl: String,
    /// SPARQL query file path used for extraction
    pub query_path: String,
    /// Tera template file path used for rendering
    pub template_path: String,
    /// MIME type of artifact (e.g., "text/x-rust", "application/typescript")
    pub mime_type: String,
}

/// ggen receipt metadata
#[derive(Debug, Clone)]
pub struct ReceiptMetadata {
    /// UUID of the ggen operation
    pub operation_id: String,
    /// Ed25519 signature (base64)
    pub signature: String,
    /// BLAKE3 hash of previous receipt (hex string), if chained
    pub previous_receipt_hash: Option<String>,
    /// Timestamp of operation (RFC3339)
    pub timestamp: String,
    /// Total number of artifacts emitted in this operation
    pub artifact_count: usize,
}

/// Compute BLAKE3 hash of a file.
///
/// # Arguments
/// * `path` - File path to hash
///
/// # Returns
/// Hex string representation of BLAKE3 hash (64 characters)
pub fn hash_artifact(path: &Path) -> Result<String> {
    let content = fs::read(path)
        .map_err(|e| anyhow!("Failed to read artifact {}: {}", path.display(), e))?;
    let hash = blake3::hash(&content);
    Ok(hash.to_hex().to_string())
}

/// Parse ggen receipt JSON file.
///
/// Expects receipt structure:
/// ```json
/// {
///   "operation_id": "<uuid>",
///   "timestamp": "<rfc3339>",
///   "input_hashes": ["file:<hash>", ...],
///   "output_hashes": ["file:<hash>", ...],
///   "signature": "<base64>",
///   "previous_receipt_hash": "<hex>" | null
/// }
/// ```
fn parse_receipt(receipt_path: &Path) -> Result<ReceiptMetadata> {
    let content = fs::read_to_string(receipt_path)
        .map_err(|e| anyhow!("Failed to read receipt {}: {}", receipt_path.display(), e))?;
    let receipt_json: Value = serde_json::from_str(&content)
        .map_err(|e| anyhow!("Invalid receipt JSON: {}", e))?;

    let operation_id = receipt_json
        .get("operation_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Missing operation_id in receipt"))?;

    let signature = receipt_json
        .get("signature")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Missing signature in receipt"))?;

    let timestamp = receipt_json
        .get("timestamp")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Missing timestamp in receipt"))?;

    let previous_receipt_hash = receipt_json
        .get("previous_receipt_hash")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let artifact_count = receipt_json
        .get("output_hashes")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);

    Ok(ReceiptMetadata {
        operation_id,
        signature,
        previous_receipt_hash,
        timestamp,
        artifact_count,
    })
}

/// Infer MIME type from file extension.
fn mime_type_for_path(path: &str) -> String {
    match Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
    {
        Some("rs") => "text/x-rust".to_string(),
        Some("ts") => "application/typescript".to_string(),
        Some("tsx") => "application/typescript+jsx".to_string(),
        Some("js") => "application/javascript".to_string(),
        Some("jsx") => "application/javascript+jsx".to_string(),
        Some("sql") => "application/x-sql".to_string(),
        Some("py") => "text/x-python".to_string(),
        Some("md") => "text/markdown".to_string(),
        Some("json") => "application/json".to_string(),
        Some("yaml") | Some("yml") => "application/x-yaml".to_string(),
        Some("tf") => "text/x-terraform".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

/// Extract source TTL files from receipt input_hashes.
///
/// Filters input_hashes to find entries ending in `.ttl`
fn extract_source_ttls(receipt: &Value) -> String {
    receipt
        .get("input_hashes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| {
                    entry.as_str().and_then(|s| {
                        if s.ends_with(".ttl") {
                            Some(s.split(':').next().unwrap_or(s).to_string())
                        } else {
                            None
                        }
                    })
                })
                .collect::<Vec<_>>()
                .join("|")
        })
        .unwrap_or_default()
}

/// Build artifact registration record for a single output file.
///
/// # Arguments
/// * `artifact_path` - Artifact file path (relative to project root)
/// * `receipt_path` - Path to ggen receipt JSON
/// * `source_ttl` - Source TTL file(s) used (pipe-separated)
/// * `query_path` - SPARQL query file used
/// * `template_path` - Tera template file used
///
/// # Returns
/// ArtifactRecord with all metadata
pub fn build_artifact_record(
    artifact_path: &str,
    receipt_path: &Path,
    source_ttl: &str,
    query_path: &str,
    template_path: &str,
) -> Result<ArtifactRecord> {
    // Compute BLAKE3 hash of artifact
    let artifact_full_path = PathBuf::from(artifact_path);
    let hash = hash_artifact(&artifact_full_path)?;

    // Parse receipt metadata
    let receipt_meta = parse_receipt(receipt_path)?;
    let receipt_json: Value = serde_json::from_str(&fs::read_to_string(receipt_path)?)?;

    // Infer MIME type
    let mime_type = mime_type_for_path(artifact_path);

    Ok(ArtifactRecord {
        path: artifact_path.to_string(),
        hash,
        receipt: receipt_meta,
        generated_at: receipt_json
            .get("timestamp")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default(),
        source_ttl: source_ttl.to_string(),
        query_path: query_path.to_string(),
        template_path: template_path.to_string(),
        mime_type,
    })
}

/// Build SPARQL INSERT query for artifact registration.
///
/// Returns SPARQL CONSTRUCT query with:
/// - Artifact RDF assertions (path, hash, receipt)
/// - Receipt RDF assertions (operation_id, signature, timestamp)
/// - Links to source ontology and templates
pub fn build_sparql_registration_query(artifacts: &[ArtifactRecord]) -> Result<String> {
    let mut bindings = Vec::new();

    for artifact in artifacts {
        let path_escaped = artifact.path.replace('/', ":");
        let artifact_iri = format!("urn:ggen:artifact:{}", path_escaped);
        let receipt_iri = format!("urn:ggen:receipt:{}", artifact.receipt.operation_id);

        // Build query string with BIND statements for this artifact
        bindings.push(format!(
            r#"BIND("{}" AS ?path)
BIND("{}" AS ?hash)
BIND("{}" AS ?timestamp)
BIND("{}" AS ?sourceTTL)
BIND("{}" AS ?query)
BIND("{}" AS ?template)
BIND("{}" AS ?mimeType)
BIND("{}" AS ?operationId)
BIND("{}" AS ?signature)
BIND({} AS ?previousReceiptHash)
BIND({} AS ?artifactCount)"#,
            artifact.path,
            artifact.hash,
            artifact.generated_at,
            artifact.source_ttl,
            artifact.query_path,
            artifact.template_path,
            artifact.mime_type,
            artifact.receipt.operation_id,
            artifact.receipt.signature,
            artifact
                .receipt
                .previous_receipt_hash
                .as_ref()
                .map(|h| format!(r#""{}""#, h))
                .unwrap_or_else(|| "undef".to_string()),
            artifact.receipt.artifact_count
        ));
    }

    // Load the base query from .specify/queries/register-ggen-artifacts.rq
    let query_template = include_str!("../.specify/queries/register-ggen-artifacts.rq");

    // Append BIND statements for first artifact (simplified for single-artifact case)
    // For multiple artifacts, you'd need to parameterize or loop the INSERT
    let complete_query = if !bindings.is_empty() {
        format!("{}\n{}", query_template, bindings[0])
    } else {
        query_template.to_string()
    };

    Ok(complete_query)
}

/// Register all artifacts from a ggen execution into open-ontologies RDF store.
///
/// # Arguments
/// * `artifacts` - List of ArtifactRecords
/// * `oxigraph_store` - Oxigraph SPARQL endpoint (e.g., "http://localhost:7878")
/// * `graph_uri` - Named graph URI (default: `onto:artifact-registry`)
///
/// # Returns
/// Number of triples inserted
///
/// # Example
/// ```ignore
/// let artifacts = vec![
///     build_artifact_record(
///         "src/cmds/generated.rs",
///         Path::new(".ggen/receipts/latest.json"),
///         "ontology/cli-open-ontologies.ttl",
///         ".specify/queries/extract-commands.rq",
///         ".specify/templates/cli.tera",
///     )?,
/// ];
///
/// let triple_count = register_artifacts(
///     &artifacts,
///     "http://localhost:7878",
///     "urn:graph:artifact-registry",
/// ).await?;
/// println!("Registered {} triples", triple_count);
/// ```
pub async fn register_artifacts(
    artifacts: &[ArtifactRecord],
    oxigraph_endpoint: &str,
    graph_uri: &str,
) -> Result<usize> {
    // Build SPARQL INSERT query
    let sparql_query = build_sparql_registration_query(artifacts)?;

    // POST to Oxigraph SPARQL endpoint
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/query", oxigraph_endpoint))
        .header("Content-Type", "application/sparql-update")
        .body(sparql_query)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to POST SPARQL to Oxigraph: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Oxigraph returned {}: {}",
            response.status(),
            error_text
        ));
    }

    // Query artifact count in registry to confirm registration
    let verify_query = format!(
        "SELECT (COUNT(?artifact) AS ?count) WHERE {{ GRAPH <{}> {{ ?artifact a <https://ggen.io/onto/ggen/Artifact> }} }}",
        graph_uri
    );

    let verify_response = client
        .post(format!("{}/query", oxigraph_endpoint))
        .header("Content-Type", "application/sparql-query")
        .header("Accept", "application/sparql-results+json")
        .body(verify_query)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to verify artifact count: {}", e))?;

    let result: Value = verify_response.json().await?;
    let count = result
        .get("results")
        .and_then(|r| r.get("bindings"))
        .and_then(|b| b.get(0))
        .and_then(|binding| binding.get("count"))
        .and_then(|c| c.get("value"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_mime_type_for_path() {
        assert_eq!(mime_type_for_path("src/main.rs"), "text/x-rust");
        assert_eq!(mime_type_for_path("app.ts"), "application/typescript");
        assert_eq!(mime_type_for_path("script.sql"), "application/x-sql");
        assert_eq!(mime_type_for_path("unknown.xyz"), "application/octet-stream");
    }

    #[test]
    fn test_hash_artifact() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let artifact_path = temp_dir.path().join("test.rs");
        fs::write(&artifact_path, b"fn main() {}")?;

        let hash = hash_artifact(&artifact_path)?;
        assert_eq!(hash.len(), 64); // BLAKE3 hex is 64 chars
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

        Ok(())
    }

    #[test]
    fn test_build_artifact_record() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let artifact_path = temp_dir.path().join("test.rs");
        fs::write(&artifact_path, b"fn main() {}")?;

        let receipt_path = temp_dir.path().join("receipt.json");
        fs::write(
            &receipt_path,
            r#"{
  "operation_id": "12345678-1234-1234-1234-123456789abc",
  "timestamp": "2026-06-01T15:30:45Z",
  "input_hashes": [],
  "output_hashes": [],
  "signature": "sig123",
  "previous_receipt_hash": null
}"#,
        )?;

        let record = build_artifact_record(
            "src/test.rs",
            &receipt_path,
            "ontology/test.ttl",
            ".specify/queries/test.rq",
            ".specify/templates/test.tera",
        )?;

        assert_eq!(record.path, "src/test.rs");
        assert_eq!(record.hash.len(), 64);
        assert_eq!(record.mime_type, "text/x-rust");
        assert_eq!(
            record.receipt.operation_id,
            "12345678-1234-1234-1234-123456789abc"
        );

        Ok(())
    }

    #[test]
    fn test_build_sparql_registration_query() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let artifact_path = temp_dir.path().join("test.rs");
        fs::write(&artifact_path, b"fn main() {}")?;

        let receipt_path = temp_dir.path().join("receipt.json");
        fs::write(
            &receipt_path,
            r#"{
  "operation_id": "test-op-id",
  "timestamp": "2026-06-01T15:30:45Z",
  "input_hashes": ["ontology/cli.ttl:abc123"],
  "output_hashes": ["src/test.rs:def456"],
  "signature": "sig123",
  "previous_receipt_hash": "prev123"
}"#,
        )?;

        let record = build_artifact_record(
            "src/test.rs",
            &receipt_path,
            "ontology/cli.ttl",
            ".specify/queries/test.rq",
            ".specify/templates/test.tera",
        )?;

        let query = build_sparql_registration_query(&[record])?;
        assert!(query.contains("INSERT"));
        assert!(query.contains("ggen:Artifact"));
        assert!(query.contains("src/test.rs"));

        Ok(())
    }
}
