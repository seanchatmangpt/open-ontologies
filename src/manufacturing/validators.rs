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
    for f in &bundle.files {
        // Every file MUST carry a receipt header (or JSON-embedded
        // receipt for non-comment-friendly formats) bound to the work
        // order.
        let has_comment_header = f.contents.contains("ostar-artifact-hash:");
        let has_json_receipt = f.contents.contains("\"_ontostar_receipt\":");
        if !has_comment_header && !has_json_receipt {
            return Err(DefectClass::ManufacturingChainBroken {
                missing: format!("receipt header on {}", f.path),
            });
        }
        if !f
            .contents
            .contains(&bundle.spec.work_order_receipt_hash)
        {
            return Err(DefectClass::ManufacturingChainBroken {
                missing: format!("work-order binding on {}", f.path),
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
    for f in iac {
        // Every IaC file is pure JSON with the receipt embedded as a
        // top-level `_ontostar_receipt` object. Both shape constraints
        // are checked here.
        let parsed: serde_json::Value = match serde_json::from_str(&f.contents) {
            Ok(v) => v,
            Err(e) => {
                return Err(DefectClass::IacInvalid {
                    reason: format!("{} is not valid JSON: {e}", f.path),
                });
            }
        };
        let receipt = parsed.get("_ontostar_receipt").ok_or_else(|| {
            DefectClass::IacInvalid {
                reason: format!("{} missing _ontostar_receipt key", f.path),
            }
        })?;
        if receipt.get("artifact_hash").and_then(|v| v.as_str()).is_none() {
            return Err(DefectClass::IacInvalid {
                reason: format!("{} _ontostar_receipt.artifact_hash missing/non-string", f.path),
            });
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
            && trimmed
                .splitn(2, ": ")
                .nth(1)
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
