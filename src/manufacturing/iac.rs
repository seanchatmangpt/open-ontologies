//! IaC generator — Terraform JSON for AWS.
//!
//! Emits clean Terraform JSON (no extraneous keys — verified by
//! `terraform validate` in adversarial test) plus a sidecar
//! `iac/.ontostar-receipt.json` that binds the bundle to its work
//! order. Receipts cannot live inside the .tf.json files because
//! Terraform's top-level schema is closed.

use super::{ManufacturedFile, SolutionSpec};

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
