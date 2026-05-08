//! IaC generator — Terraform JSON for AWS.
//!
//! Emits `iac/main.tf.json` (a Terraform plan in JSON form, not HCL —
//! valid Terraform input that does not need an HCL parser to validate)
//! plus `iac/variables.tf.json` and `iac/outputs.tf.json`. Deterministic
//! over the SolutionSpec.

use super::{ManufacturedFile, SolutionSpec};

pub fn generate(spec: &SolutionSpec) -> Vec<ManufacturedFile> {
    if spec.iac_target != "aws" {
        return Vec::new();
    }
    vec![
        file("iac/main.tf.json", generate_main(spec), spec),
        file("iac/variables.tf.json", generate_variables(spec), spec),
        file("iac/outputs.tf.json", generate_outputs(spec), spec),
    ]
}

/// Build a Terraform-JSON file. The receipt header is injected as a
/// top-level `_ontostar_receipt` JSON key (Terraform ignores unknown
/// top-level blocks during plan; for strict mode the verifier may
/// strip this key before passing to `terraform validate`). Comment-
/// style headers cannot appear in JSON, so the binding goes inside.
fn file(path: &str, body_json: serde_json::Value, spec: &SolutionSpec) -> ManufacturedFile {
    let mut obj = match body_json {
        serde_json::Value::Object(m) => m,
        other => {
            // Generator always returns an object; defensive fallback
            // wraps the value so we still have a valid JSON file.
            let mut m = serde_json::Map::new();
            m.insert("body".into(), other);
            m
        }
    };
    let body_for_hash =
        serde_json::to_string(&serde_json::Value::Object(obj.clone())).unwrap_or_default();
    let artifact_hash = blake3::hash(body_for_hash.as_bytes()).to_hex().to_string();
    obj.insert(
        "_ontostar_receipt".into(),
        serde_json::json!({
            "production_law": "ontostar-1.0.0",
            "defects_taxonomy": crate::defects::DEFECTS_TAXONOMY_VERSION,
            "target": "iac",
            "artifact_hash": artifact_hash,
            "work_order_receipt": spec.work_order_receipt_hash,
            "solution_name": spec.name,
        }),
    );
    let contents = serde_json::to_string_pretty(&serde_json::Value::Object(obj))
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
