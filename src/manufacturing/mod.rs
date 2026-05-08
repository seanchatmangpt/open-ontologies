//! Solution Manufacturing — Phase 4.
//!
//! Given a `SolutionSpec` derived from an admitted work order, deterministi-
//! cally generate a coherent multi-target stack:
//!
//!   - **IaC** (Terraform JSON) — infrastructure declaration
//!   - **Rust** — high-performance service code
//!   - **Erlang/OTP** — supervision tree / gen_server orchestration
//!   - **AtomVM** — embedded Erlang for IoT / edge nodes
//!
//! Every emitted file carries an OntoStar receipt header so external
//! verifiers can strip-and-rehash to prove provenance. Generators are
//! pure functions over the `SolutionSpec` — same input always produces
//! byte-identical output (deterministic templates, no time / random).

use serde::{Deserialize, Serialize};

pub mod atomvm;
pub mod erlang;
pub mod iac;
pub mod rust_target;
pub mod validators;

/// What the caller wants manufactured. Derived from the admitted work
/// order: the work order's CTQ + counterfactual + a small set of
/// architectural knobs.
///
/// All fields are bounded strings — no IRIs, no secrets, no raw user
/// data. The generators trust the spec to be sanitized at admission
/// time (the gate enforces RawDataLeak / SecretLeak before any
/// generator is called).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionSpec {
    /// Project / solution name. Becomes the Terraform module name, the
    /// Rust crate name, the Erlang application name, the AtomVM module
    /// name. Must be a valid identifier (`[a-z][a-z0-9_]*`).
    pub name: String,
    /// One-sentence description of the solution. Embedded as a comment
    /// in every generated file.
    pub description: String,
    /// Cloud / runtime target for the IaC. Currently only "aws" is
    /// wired; other values produce an empty IaC bundle which the gate
    /// rejects with `IacInvalid{reason}`.
    pub iac_target: String,
    /// AWS region (or analogous) for IaC. Ignored when `iac_target`
    /// is not "aws".
    pub region: String,
    /// Number of supervisor children for the Erlang/OTP tree.
    pub supervisor_children: u32,
    /// Microcontroller target for AtomVM. One of "esp32", "stm32",
    /// "rp2040". Other values produce empty AtomVM output and the
    /// gate rejects with `AtomVmInvalid{reason}`.
    pub mcu_target: String,
    /// Receipt hash from the upstream WorkOrderAdmitted receipt.
    /// Embedded in every generated file's header so the entire
    /// stack is provably bound to one work order. Empty string is
    /// rejected as `ArchitectureUnbound`.
    pub work_order_receipt_hash: String,
}

/// One file in the manufactured bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManufacturedFile {
    /// Relative path within the bundle (e.g. `iac/main.tf.json`,
    /// `rust/src/lib.rs`, `erlang/src/sup.erl`, `atomvm/main.erl`).
    pub path: String,
    /// File contents as UTF-8 text. Generators may not emit binary —
    /// every artifact is human-readable so the receipt header round-
    /// trip is exercisable.
    pub contents: String,
    /// Logical target this file belongs to: "iac" / "rust" / "erlang"
    /// / "atomvm".
    pub target: String,
}

/// Complete manufactured bundle for a single SolutionSpec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionBundle {
    pub spec: SolutionSpec,
    pub files: Vec<ManufacturedFile>,
}

impl SolutionBundle {
    /// Total bytes across all files.
    pub fn total_bytes(&self) -> usize {
        self.files.iter().map(|f| f.contents.len()).sum()
    }

    /// Files for a given target.
    pub fn files_for(&self, target: &str) -> Vec<&ManufacturedFile> {
        self.files.iter().filter(|f| f.target == target).collect()
    }
}

/// Validate a `SolutionSpec` deterministically. Returns `Ok(())` when
/// the spec is acceptable; otherwise returns the typed defect that
/// the admission gate will surface.
pub fn validate_spec(spec: &SolutionSpec) -> Result<(), crate::defects::DefectClass> {
    use crate::defects::DefectClass;
    if spec.work_order_receipt_hash.trim().is_empty() {
        return Err(DefectClass::ArchitectureUnbound);
    }
    let h = spec.work_order_receipt_hash.trim();
    if h.len() != 64 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(DefectClass::ArchitectureUnbound);
    }
    if spec.name.trim().is_empty()
        || !spec
            .name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        || !spec
            .name
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic())
            .unwrap_or(false)
    {
        return Err(DefectClass::IacInvalid {
            reason: "name must match [a-z][a-z0-9_]*".into(),
        });
    }
    if !matches!(spec.iac_target.as_str(), "aws") {
        return Err(DefectClass::IacInvalid {
            reason: format!("unsupported iac_target: {}", spec.iac_target),
        });
    }
    if !matches!(spec.mcu_target.as_str(), "esp32" | "stm32" | "rp2040") {
        return Err(DefectClass::AtomVmInvalid {
            reason: format!("unsupported mcu_target: {}", spec.mcu_target),
        });
    }
    if spec.supervisor_children == 0 || spec.supervisor_children > 64 {
        return Err(DefectClass::ErlangInvalid {
            reason: format!(
                "supervisor_children must be in [1, 64]; got {}",
                spec.supervisor_children
            ),
        });
    }
    Ok(())
}

/// Build the full multi-target bundle. Every generator is invoked; if
/// any returns an empty file list, the corresponding `*Invalid`
/// defect is returned.
pub fn manufacture(spec: &SolutionSpec) -> Result<SolutionBundle, crate::defects::DefectClass> {
    use crate::defects::DefectClass;
    validate_spec(spec)?;
    let mut files = Vec::new();
    let iac_files = iac::generate(spec);
    if iac_files.is_empty() {
        return Err(DefectClass::GeneratorEmpty { target: "iac".into() });
    }
    files.extend(iac_files);

    let rust_files = rust_target::generate(spec);
    if rust_files.is_empty() {
        return Err(DefectClass::GeneratorEmpty { target: "rust".into() });
    }
    files.extend(rust_files);

    let erlang_files = erlang::generate(spec);
    if erlang_files.is_empty() {
        return Err(DefectClass::GeneratorEmpty {
            target: "erlang".into(),
        });
    }
    files.extend(erlang_files);

    let atomvm_files = atomvm::generate(spec);
    if atomvm_files.is_empty() {
        return Err(DefectClass::GeneratorEmpty {
            target: "atomvm".into(),
        });
    }
    files.extend(atomvm_files);

    let bundle = SolutionBundle {
        spec: spec.clone(),
        files,
    };
    validators::validate_bundle(&bundle)?;
    Ok(bundle)
}

/// Comment prefix appropriate for a generated file. Used by every
/// generator's header builder so the receipt-header rule is uniform.
pub(crate) fn comment_prefix_for(path: &str) -> &'static str {
    if path.ends_with(".rs") || path.ends_with(".tf") {
        "//"
    } else if path.ends_with(".erl") || path.ends_with(".hrl") {
        "%%"
    } else {
        "#"
    }
}

/// Build a 6-line receipt header bound to the work-order receipt.
/// External verifiers strip every leading line matching `^<prefix> ostar-[a-z-]+: .+$`,
/// hash the remainder, and assert equality with `ostar-artifact-hash`.
pub(crate) fn receipt_header(spec: &SolutionSpec, path: &str, body: &str) -> String {
    let prefix = comment_prefix_for(path);
    let artifact_hash = blake3::hash(body.as_bytes()).to_hex().to_string();
    format!(
        "{p} ostar-production-law: ontostar-1.0.0\n\
         {p} ostar-defects-taxonomy: {tax}\n\
         {p} ostar-target: {target}\n\
         {p} ostar-artifact-hash: {ah}\n\
         {p} ostar-work-order-receipt: {wor}\n\
         {p} ostar-solution-name: {name}\n",
        p = prefix,
        tax = crate::defects::DEFECTS_TAXONOMY_VERSION,
        target = path.split('/').next().unwrap_or("unknown"),
        ah = artifact_hash,
        wor = spec.work_order_receipt_hash,
        name = spec.name,
    )
}

/// Wrap a freshly-generated `body` with its receipt header.
pub(crate) fn with_header(spec: &SolutionSpec, path: &str, body: &str) -> String {
    let mut out = receipt_header(spec, path, body);
    out.push_str(body);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_spec() -> SolutionSpec {
        SolutionSpec {
            name: "revops_pipeline".into(),
            description: "Fortune-5 RevOps pipeline manufactured stack".into(),
            iac_target: "aws".into(),
            region: "us-east-1".into(),
            supervisor_children: 4,
            mcu_target: "esp32".into(),
            work_order_receipt_hash: "0".repeat(64),
        }
    }

    #[test]
    fn validate_spec_accepts_canonical() {
        validate_spec(&ok_spec()).expect("ok spec should validate");
    }

    #[test]
    fn validate_spec_rejects_empty_receipt() {
        let mut s = ok_spec();
        s.work_order_receipt_hash = "".into();
        assert!(matches!(
            validate_spec(&s),
            Err(crate::defects::DefectClass::ArchitectureUnbound)
        ));
    }

    #[test]
    fn validate_spec_rejects_bad_name() {
        let mut s = ok_spec();
        s.name = "1bad".into();
        assert!(matches!(
            validate_spec(&s),
            Err(crate::defects::DefectClass::IacInvalid { .. })
        ));
    }

    #[test]
    fn validate_spec_rejects_unsupported_mcu() {
        let mut s = ok_spec();
        s.mcu_target = "msp430".into();
        assert!(matches!(
            validate_spec(&s),
            Err(crate::defects::DefectClass::AtomVmInvalid { .. })
        ));
    }

    #[test]
    fn validate_spec_rejects_supervisor_zero() {
        let mut s = ok_spec();
        s.supervisor_children = 0;
        assert!(matches!(
            validate_spec(&s),
            Err(crate::defects::DefectClass::ErlangInvalid { .. })
        ));
    }

    #[test]
    fn manufacture_emits_all_four_targets() {
        let bundle = manufacture(&ok_spec()).expect("manufacture must succeed");
        assert!(!bundle.files_for("iac").is_empty());
        assert!(!bundle.files_for("rust").is_empty());
        assert!(!bundle.files_for("erlang").is_empty());
        assert!(!bundle.files_for("atomvm").is_empty());
    }

    #[test]
    fn manufacture_is_deterministic() {
        let a = manufacture(&ok_spec()).unwrap();
        let b = manufacture(&ok_spec()).unwrap();
        assert_eq!(a.total_bytes(), b.total_bytes());
        for (x, y) in a.files.iter().zip(b.files.iter()) {
            assert_eq!(x.path, y.path);
            assert_eq!(x.contents, y.contents);
        }
    }

    #[test]
    fn every_file_carries_receipt_header() {
        let bundle = manufacture(&ok_spec()).unwrap();
        for f in &bundle.files {
            // IaC files use a JSON-embedded receipt (top-level
            // `_ontostar_receipt`); the other targets use a comment-
            // prefixed receipt header. Either form is accepted.
            let has_comment = f.contents.contains("ostar-artifact-hash:");
            let has_json = f.contents.contains("\"_ontostar_receipt\":");
            assert!(
                has_comment || has_json,
                "{} missing receipt binding",
                f.path
            );
            assert!(
                f.contents.contains(&bundle.spec.work_order_receipt_hash),
                "{} missing work-order receipt hash",
                f.path
            );
        }
    }
}
