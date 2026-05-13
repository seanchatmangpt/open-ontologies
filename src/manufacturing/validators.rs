//! Cross-target deterministic validators.
//!
//! Run AFTER each generator emits its files but BEFORE the bundle is
//! returned from `manufacture()`. Validation is byte-level and target-
//! specific: every defect class in src/defects.rs that mentions a
//! generator has a corresponding check here.

use super::{ManufacturedFile, SolutionBundle};
use crate::defects::DefectClass;

/// Validate the bundle. Returns the first defect found, or Ok(()) if
/// every file in every target passes.
pub fn validate_bundle(bundle: &SolutionBundle) -> Result<(), DefectClass> {
    // Two binding forms are accepted:
    //   - comment-prefixed `ostar-artifact-hash:` header (Rust / Erlang
    //     / AtomVM source files)
    //   - separate sidecar `iac/.ontostar-receipt.json` that names the
    //     bound files (Terraform JSON, which has a closed top-level
    //     schema and cannot embed receipts)
    let iac_files: std::collections::HashSet<String> = bundle
        .files_for("iac")
        .iter()
        .filter(|f| f.path.ends_with(".tf.json"))
        .map(|f| {
            std::path::Path::new(&f.path)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
        .collect();
    let sidecar_files: std::collections::HashSet<String> = bundle
        .files
        .iter()
        .find(|f| f.path == "iac/.ontostar-receipt.json")
        .and_then(|f| serde_json::from_str::<serde_json::Value>(&f.contents).ok())
        .and_then(|v| v.get("files").cloned())
        .and_then(|v| v.as_array().cloned())
        .map(|arr| {
            arr.into_iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    for f in &bundle.files {
        let has_comment_header = f.contents.contains("ostar-artifact-hash:");
        let basename = std::path::Path::new(&f.path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let is_sidecar_bound = f.target == "iac"
            && f.path.ends_with(".tf.json")
            && sidecar_files.contains(&basename);
        let is_the_sidecar = f.path == "iac/.ontostar-receipt.json";
        if !has_comment_header && !is_sidecar_bound && !is_the_sidecar {
            return Err(DefectClass::ManufacturingChainBroken {
                missing: format!("receipt binding on {}", f.path),
            });
        }
    }
    // The sidecar (and only the sidecar) carries the work-order receipt
    // for the IaC bundle. Other targets carry it inline. We assert the
    // work_order_receipt_hash appears at least once across the bundle.
    let bundle_blob = bundle
        .files
        .iter()
        .map(|f| f.contents.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if !bundle_blob.contains(&bundle.spec.work_order_receipt_hash) {
        return Err(DefectClass::ManufacturingChainBroken {
            missing: "work-order receipt hash absent from entire bundle".into(),
        });
    }
    // All non-receipt iac files should be referenced by the sidecar.
    for name in &iac_files {
        if !sidecar_files.contains(name) {
            return Err(DefectClass::IacInvalid {
                reason: format!(".ontostar-receipt.json does not list {}", name),
            });
        }
    }

    validate_iac(bundle)?;
    validate_rust(bundle)?;
    validate_erlang(bundle)?;
    validate_atomvm(bundle)?;
    Ok(())
}

fn validate_iac(bundle: &SolutionBundle) -> Result<(), DefectClass> {
    let iac = bundle.files_for("iac");
    let mut have_main = false;
    let mut have_vars = false;
    let mut have_outs = false;
    let mut have_sidecar = false;
    for f in iac {
        // .tf.json files are pure Terraform JSON — no extraneous keys.
        // The receipt lives in iac/.ontostar-receipt.json (sidecar)
        // because Terraform's top-level schema is closed and any extra
        // key fails `terraform validate`.
        if f.path.ends_with(".tf.json") {
            let parsed: serde_json::Value = match serde_json::from_str(&f.contents) {
                Ok(v) => v,
                Err(e) => {
                    return Err(DefectClass::IacInvalid {
                        reason: format!("{} is not valid JSON: {e}", f.path),
                    });
                }
            };
            // Refuse Terraform JSON containing an `_ontostar_receipt`
            // key — that is the bug the adversarial audit caught and
            // it would fail `terraform validate`.
            if let Some(obj) = parsed.as_object()
                && obj.contains_key("_ontostar_receipt") {
                    return Err(DefectClass::IacInvalid {
                        reason: format!(
                            "{} contains _ontostar_receipt key — Terraform top-level schema is closed; receipts must live in the sidecar",
                            f.path
                        ),
                    });
                }
        }
        if f.path == "iac/.ontostar-receipt.json" {
            have_sidecar = true;
            let parsed: serde_json::Value = match serde_json::from_str(&f.contents) {
                Ok(v) => v,
                Err(e) => {
                    return Err(DefectClass::IacInvalid {
                        reason: format!("sidecar is not valid JSON: {e}"),
                    });
                }
            };
            if parsed.get("artifact_hash").and_then(|v| v.as_str()).is_none() {
                return Err(DefectClass::IacInvalid {
                    reason: "sidecar.artifact_hash missing/non-string".into(),
                });
            }
            if parsed.get("work_order_receipt").and_then(|v| v.as_str()).is_none() {
                return Err(DefectClass::IacInvalid {
                    reason: "sidecar.work_order_receipt missing/non-string".into(),
                });
            }
        }
        if f.path.ends_with("main.tf.json") {
            have_main = true;
        }
        if f.path.ends_with("variables.tf.json") {
            have_vars = true;
        }
        if f.path.ends_with("outputs.tf.json") {
            have_outs = true;
        }
    }
    if !have_sidecar {
        return Err(DefectClass::IacInvalid {
            reason: "iac/.ontostar-receipt.json sidecar is missing".into(),
        });
    }
    if !(have_main && have_vars && have_outs) {
        return Err(DefectClass::IacInvalid {
            reason: format!(
                "expected main+variables+outputs; got main={have_main} vars={have_vars} outs={have_outs}"
            ),
        });
    }
    Ok(())
}

fn validate_rust(bundle: &SolutionBundle) -> Result<(), DefectClass> {
    let rust = bundle.files_for("rust");
    let mut have_cargo = false;
    let mut have_lib = false;
    let mut have_main = false;
    for f in rust {
        if f.path.ends_with("Cargo.toml") {
            have_cargo = true;
            if !f.contents.contains("[package]") {
                return Err(DefectClass::RustInvalid {
                    reason: "Cargo.toml missing [package]".into(),
                });
            }
        }
        if f.path.ends_with("lib.rs") {
            have_lib = true;
            if !f.contents.contains("pub fn manufactured_solution_name") {
                return Err(DefectClass::RustInvalid {
                    reason: "lib.rs missing manufactured_solution_name".into(),
                });
            }
        }
        if f.path.ends_with("main.rs") {
            have_main = true;
            if !f.contents.contains("fn main") {
                return Err(DefectClass::RustInvalid {
                    reason: "main.rs missing fn main".into(),
                });
            }
        }
    }
    if !(have_cargo && have_lib && have_main) {
        return Err(DefectClass::RustInvalid {
            reason: format!(
                "expected Cargo+lib+main; got Cargo={have_cargo} lib={have_lib} main={have_main}"
            ),
        });
    }
    Ok(())
}

fn validate_erlang(bundle: &SolutionBundle) -> Result<(), DefectClass> {
    let erlang = bundle.files_for("erlang");
    let mut have_app = false;
    let mut have_sup = false;
    let mut have_worker = false;
    let mut have_rebar = false;
    for f in erlang {
        if f.path.ends_with("_app.erl") {
            have_app = true;
            // The body (after stripping the `%%` receipt header) MUST
            // contain `-module(` and `-export(`. Both are required for
            // the file to be a real Erlang module.
            require_erlang_decl(f, "-module(")?;
            require_erlang_decl(f, "-export(")?;
        }
        if f.path.ends_with("_sup.erl") {
            have_sup = true;
            require_erlang_decl(f, "-behaviour(supervisor)")?;
        }
        if f.path.ends_with("_worker.erl") {
            have_worker = true;
            require_erlang_decl(f, "-behaviour(gen_server)")?;
        }
        if f.path.ends_with("rebar.config") {
            have_rebar = true;
        }
    }
    if !(have_app && have_sup && have_worker && have_rebar) {
        return Err(DefectClass::ErlangInvalid {
            reason: format!(
                "expected app+sup+worker+rebar; got app={have_app} sup={have_sup} worker={have_worker} rebar={have_rebar}"
            ),
        });
    }
    Ok(())
}

fn validate_atomvm(bundle: &SolutionBundle) -> Result<(), DefectClass> {
    let avm = bundle.files_for("atomvm");
    let mut have_module = false;
    let mut have_makefile = false;
    for f in avm {
        if f.path.ends_with(".erl") {
            have_module = true;
            require_erlang_decl(f, "-module(")?;
            // AtomVM entry point.
            if !f.contents.contains("start() ->") && !f.contents.contains("start()->") {
                return Err(DefectClass::AtomVmInvalid {
                    reason: format!("{} missing start/0 entry point", f.path),
                });
            }
        }
        if f.path.ends_with("Makefile") {
            have_makefile = true;
            if !f.contents.contains(".avm:") {
                return Err(DefectClass::AtomVmInvalid {
                    reason: "Makefile missing .avm build target".into(),
                });
            }
        }
    }
    if !(have_module && have_makefile) {
        return Err(DefectClass::AtomVmInvalid {
            reason: format!(
                "expected module.erl + Makefile; got module={have_module} makefile={have_makefile}"
            ),
        });
    }
    Ok(())
}

fn require_erlang_decl(file: &ManufacturedFile, needle: &str) -> Result<(), DefectClass> {
    if !file.contents.contains(needle) {
        return Err(DefectClass::ErlangInvalid {
            reason: format!("{} missing `{needle}` declaration", file.path),
        });
    }
    Ok(())
}

/// Strip every leading line matching `^<prefix> ostar-[a-z-]+: .+$`
/// from the contents. Returns the remaining body. Used by external
/// verifiers to recover the receipt-bound payload from a generated
/// file. (Not used internally by the validators — they check the
/// header is present, not absent. This helper exists for tests and
/// for external auditors.)
pub fn strip_header(contents: &str, prefix: &str) -> String {
    let header_marker = format!("{prefix} ostar-");
    let mut body_start = 0usize;
    for line in contents.split_inclusive('\n') {
        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        let is_header = trimmed.starts_with(&header_marker)
            && trimmed.split_once(": ").map(|x| x.1)
                .map(|v| !v.is_empty())
                .unwrap_or(false);
        if is_header {
            body_start += line.len();
        } else {
            break;
        }
    }
    contents[body_start..].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_header_removes_only_ostar_lines() {
        let s = "// ostar-production-law: x\n// ostar-artifact-hash: deadbeef\n\
                 fn main() {}\n";
        let body = strip_header(s, "//");
        assert_eq!(body, "fn main() {}\n");
    }

    #[test]
    fn strip_header_no_header_returns_input() {
        let s = "fn main() {}\n";
        assert_eq!(strip_header(s, "//"), s);
    }

    #[test]
    fn strip_header_handles_erlang_double_percent() {
        let s = "%% ostar-production-law: x\n%% ostar-artifact-hash: y\n\
                 -module(foo).\n";
        let body = strip_header(s, "%%");
        assert_eq!(body, "-module(foo).\n");
    }
}
