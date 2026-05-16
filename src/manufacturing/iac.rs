//! IaC generator — Terraform JSON for AWS.
//!
//! Emits clean Terraform JSON (no extraneous keys — verified by
//! `terraform validate` in adversarial test) plus a sidecar
//! `iac/.ontostar-receipt.json` that binds the bundle to its work
//! order. Receipts cannot live inside the .tf.json files because
//! Terraform's top-level schema is closed.

use super::{ManufacturedFile, SolutionSpec};

/// Generate Terraform JSON files for AWS plus the OntoStar sidecar receipt.
///
/// Returns `main.tf.json`, `variables.tf.json`, `outputs.tf.json`, and
/// `iac/.ontostar-receipt.json` when `iac_target == "aws"`. Returns an
/// empty list for any other target; the admission gate treats that as
/// [`DefectClass::GeneratorEmpty`].
///
/// The three `.tf.json` files contain **no** `_ontostar_receipt` key —
/// Terraform's top-level schema is closed and any extra key fails
/// `terraform validate`. The work-order binding lives exclusively in the
/// sidecar.
///
/// # Examples
///
/// ```
/// use open_ontologies::manufacturing::{iac, SolutionSpec};
///
/// let spec = SolutionSpec {
///     name: "infra_prod".into(),
///     description: "Production infrastructure".into(),
///     iac_target: "aws".into(),
///     region: "us-west-2".into(),
///     supervisor_children: 2,
///     mcu_target: "esp32".into(),
///     work_order_receipt_hash: "d".repeat(64),
/// };
/// let files = iac::generate(&spec);
/// assert_eq!(files.len(), 4);
///
/// // Three Terraform JSON files are syntactically valid JSON.
/// let tf_files: Vec<_> = files.iter().filter(|f| f.path.ends_with(".tf.json")).collect();
/// assert_eq!(tf_files.len(), 3);
/// for f in &tf_files {
///     assert!(serde_json::from_str::<serde_json::Value>(&f.contents).is_ok());
///     // No inline receipt key — Terraform schema is closed.
///     assert!(!f.contents.contains("_ontostar_receipt"));
/// }
///
/// // Sidecar receipt carries the work-order hash.
/// let sidecar = files.iter().find(|f| f.path == "iac/.ontostar-receipt.json").unwrap();
/// let v: serde_json::Value = serde_json::from_str(&sidecar.contents).unwrap();
/// assert_eq!(v["work_order_receipt"].as_str(), Some("d".repeat(64).as_str()));
/// ```
///
/// Non-AWS targets yield no files:
///
/// ```
/// use open_ontologies::manufacturing::{iac, SolutionSpec};
///
/// let spec = SolutionSpec {
///     name: "infra_gcp".into(),
///     description: "GCP infra".into(),
///     iac_target: "gcp".into(),
///     region: "us-central1".into(),
///     supervisor_children: 1,
///     mcu_target: "esp32".into(),
///     work_order_receipt_hash: "e".repeat(64),
/// };
/// assert!(iac::generate(&spec).is_empty());
/// ```
///
/// Auto-instinct: every file in the bundle carries `target == "iac"` and
/// the three `.tf.json` files must be parseable as JSON:
///
/// ```
/// use open_ontologies::manufacturing::{iac, SolutionSpec};
///
/// let spec = SolutionSpec {
///     name: "audit_stack".into(),
///     description: "Audit infrastructure".into(),
///     iac_target: "aws".into(),
///     region: "ap-southeast-1".into(),
///     supervisor_children: 2,
///     mcu_target: "esp32".into(),
///     work_order_receipt_hash: "7".repeat(64),
/// };
/// let files = iac::generate(&spec);
/// // All four files belong to the "iac" target.
/// assert!(files.iter().all(|f| f.target == "iac"));
/// // All tf.json files are valid JSON.
/// for f in files.iter().filter(|f| f.path.ends_with(".tf.json")) {
///     let parsed = serde_json::from_str::<serde_json::Value>(&f.contents);
///     assert!(parsed.is_ok(), "{} must be valid JSON", f.path);
/// }
/// ```
///
/// Auto-instinct: generation is deterministic — identical spec produces
/// byte-identical output on repeated calls:
///
/// ```
/// use open_ontologies::manufacturing::{iac, SolutionSpec};
///
/// let spec = SolutionSpec {
///     name: "repro_check".into(),
///     description: "Determinism check".into(),
///     iac_target: "aws".into(),
///     region: "us-east-2".into(),
///     supervisor_children: 1,
///     mcu_target: "stm32".into(),
///     work_order_receipt_hash: "9".repeat(64),
/// };
/// let first  = iac::generate(&spec);
/// let second = iac::generate(&spec);
/// assert_eq!(first.len(), second.len());
/// for (a, b) in first.iter().zip(second.iter()) {
///     assert_eq!(a.path,     b.path,     "paths must match");
///     assert_eq!(a.contents, b.contents, "contents must be byte-identical");
/// }
/// ```
pub fn generate(spec: &SolutionSpec) -> Vec<ManufacturedFile> {
    if spec.iac_target != "aws" {
        return Vec::new();
    }
    let main_file = tf_file("iac/main.tf.json", generate_main(spec));
    let vars_file = tf_file("iac/variables.tf.json", generate_variables(spec));
    let outs_file = tf_file("iac/outputs.tf.json", generate_outputs(spec));
    // Sidecar receipt that binds the three Terraform JSON bodies to
    // the work order WITHOUT introducing an unknown key into
    // Terraform's strict top-level schema. The sidecar is not a
    // Terraform file — it is OntoStar metadata the verifier loads
    // alongside the bundle.
    let body_for_hash = format!(
        "{}\n{}\n{}",
        main_file.contents, vars_file.contents, outs_file.contents
    );
    let bundle_hash = blake3::hash(body_for_hash.as_bytes()).to_hex().to_string();
    let receipt = serde_json::json!({
        "production_law": "ontostar-1.0.0",
        "defects_taxonomy": crate::defects::DEFECTS_TAXONOMY_VERSION,
        "target": "iac",
        "artifact_hash": bundle_hash,
        "work_order_receipt": spec.work_order_receipt_hash,
        "solution_name": spec.name,
        "files": ["main.tf.json", "variables.tf.json", "outputs.tf.json"],
    });
    let sidecar = ManufacturedFile {
        path: "iac/.ontostar-receipt.json".to_string(),
        contents: serde_json::to_string_pretty(&receipt).expect("sidecar serializes"),
        target: "iac".to_string(),
    };
    vec![main_file, vars_file, outs_file, sidecar]
}

/// Build a clean Terraform-JSON file with NO extra keys. Terraform's
/// top-level schema is closed (terraform / provider / resource /
/// variable / output / data / module / locals); any other key fails
/// `terraform validate` with "Extraneous JSON object property". The
/// receipt lives in a sidecar.
fn tf_file(path: &str, body_json: serde_json::Value) -> ManufacturedFile {
    let contents = serde_json::to_string_pretty(&body_json)
        .expect("Terraform JSON serializes");
    ManufacturedFile {
        path: path.to_string(),
        contents,
        target: "iac".to_string(),
    }
}

fn generate_main(spec: &SolutionSpec) -> serde_json::Value {
    serde_json::json!({
        "terraform": {
            "required_version": ">= 1.5.0",
            "required_providers": {
                "aws": {
                    "source": "hashicorp/aws",
                    "version": "~> 5.0"
                }
            }
        },
        "provider": {
            "aws": {
                "region": spec.region
            }
        },
        "resource": {
            "aws_s3_bucket": {
                spec.name.clone(): {
                    "bucket": format!("{}-{}-state", spec.name, spec.region),
                    "tags": {
                        "Name": spec.name,
                        "ManagedBy": "OntoStar",
                        "WorkOrderReceipt": spec.work_order_receipt_hash
                    }
                }
            },
            "aws_dynamodb_table": {
                format!("{}_lock", spec.name): {
                    "name": format!("{}-lock", spec.name),
                    "billing_mode": "PAY_PER_REQUEST",
                    "hash_key": "LockID",
                    "attribute": [{ "name": "LockID", "type": "S" }]
                }
            }
        }
    })
}

fn generate_variables(spec: &SolutionSpec) -> serde_json::Value {
    serde_json::json!({
        "variable": {
            "region": {
                "type": "string",
                "default": spec.region,
                "description": format!("AWS region for {}", spec.name)
            },
            "solution_name": {
                "type": "string",
                "default": spec.name,
                "description": "OntoStar-bound solution identifier"
            }
        }
    })
}

fn generate_outputs(spec: &SolutionSpec) -> serde_json::Value {
    serde_json::json!({
        "output": {
            "state_bucket": {
                "value": format!("${{aws_s3_bucket.{}.bucket}}", spec.name),
                "description": "Bucket holding the OntoStar-bound Terraform state"
            },
            "work_order_receipt": {
                "value": spec.work_order_receipt_hash,
                "description": "Receipt hash binding this stack to its admitted work order"
            }
        }
    })
}
