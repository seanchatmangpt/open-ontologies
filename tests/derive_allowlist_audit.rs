//! R6 WB — derive allowlist audit.
//!
//! Closes the §22 attack vector where adding a new `#[derive(...)]` on
//! an authority/admission/receipt type could create a contract-drift
//! bypass. Example: a `#[derive(From)]` on `Receipt` would let a
//! `&str → Receipt` conversion construct a Receipt without going
//! through the receipt builder, bypassing the chain.
//!
//! This test enumerates every `#[derive(...)]` `Meta::Path` in `src/`
//! that targets an authority type and fails on un-allowlisted derives.
//! Allowed derives are the standard Rust derives that have no
//! authority-relevant side effects.

use std::collections::HashSet;
use std::path::Path;
use syn::{Attribute, Item};

const AUTHORITY_TYPES: &[&str] = &[
    "AdmissionDecision",
    "Receipt",
    "ReceiptChain",
    "Cell8Gate",
    "TrustSet",
    "AdmissionOp",
    "EvaluateAdmissionInput",
];

const ALLOWED_DERIVES: &[&str] = &[
    "Debug",
    "Clone",
    "Copy",
    "Serialize",
    "Deserialize",
    "PartialEq",
    "Eq",
    "Hash",
    "Default",
    "JsonSchema",
    "PartialOrd",
    "Ord",
];

#[test]
fn derives_on_authority_types_are_allowlisted() {
    let allowlist: HashSet<&'static str> = ALLOWED_DERIVES.iter().copied().collect();
    let authority: HashSet<&'static str> = AUTHORITY_TYPES.iter().copied().collect();

    let mut violations: Vec<String> = Vec::new();
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    walk_rs(&src_dir, &mut |path: &Path, src: &str| {
        let Ok(file) = syn::parse_file(src) else {
            return;
        };
        for item in &file.items {
            let (ident, attrs): (&syn::Ident, &[Attribute]) = match item {
                Item::Struct(s) => (&s.ident, &s.attrs),
                Item::Enum(e) => (&e.ident, &e.attrs),
                _ => continue,
            };
            let name = ident.to_string();
            if !authority.contains(name.as_str()) {
                continue;
            }
            for attr in attrs {
                if !attr.path().is_ident("derive") {
                    continue;
                }
                let _ = attr.parse_nested_meta(|m| {
                    if let Some(seg) = m.path.segments.last() {
                        let derive_name = seg.ident.to_string();
                        if !allowlist.contains(derive_name.as_str()) {
                            violations.push(format!(
                                "{}: type {} derives un-allowlisted {} \
                                 (add to allowed_derives only after auditing for \
                                 §22 contract drift)",
                                path.display(),
                                name,
                                derive_name
                            ));
                        }
                    }
                    Ok(())
                });
            }
        }
    });
    assert!(
        violations.is_empty(),
        "Derive allowlist violations on authority types:\n{}",
        violations.join("\n")
    );
}

fn walk_rs(dir: &Path, cb: &mut dyn FnMut(&Path, &str)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || name == "target" {
            continue;
        }
        if p.is_dir() {
            walk_rs(&p, cb);
        } else if p.extension().and_then(|e| e.to_str()) == Some("rs")
            && let Ok(contents) = std::fs::read_to_string(&p)
        {
            cb(&p, &contents);
        }
    }
}
