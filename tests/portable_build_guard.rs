//! Portable-build guard (v26.9.26).
//!
//! origin/main at 50eca4aa did not build from a clean checkout: the upstream
//! merge f146c229 left a corrupt `Cargo.lock` (oxrdf 0.3.3, oxttl 0.2.3 and
//! wit-bindgen 0.57.1 each listed twice), duplicate and missing `pub mod`
//! declarations in `src/lib.rs`, and `/Users/sac/...` path dependencies
//! including the no-longer-existing `wasm4pm-algos`. Each test below reads the
//! real files of this checkout and refuses one of those failure classes, so a
//! future bad merge fails here with a named reason instead of downstream in
//! Docker/cascade CI with `failed to parse lock file`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

fn manifest() -> toml::Value {
    read("Cargo.toml").parse().expect("Cargo.toml parses")
}

fn dependency_tables(m: &toml::Value) -> Vec<(String, toml::value::Table)> {
    let mut out = Vec::new();
    for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(t) = m.get(key).and_then(|v| v.as_table()) {
            out.push((key.to_string(), t.clone()));
        }
    }
    if let Some(targets) = m.get("target").and_then(|v| v.as_table()) {
        for (cfg, body) in targets {
            for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(t) = body.get(key).and_then(|v| v.as_table()) {
                    out.push((format!("target.{cfg}.{key}"), t.clone()));
                }
            }
        }
    }
    out
}

#[test]
fn manifest_has_no_absolute_or_workstation_path_dependencies() {
    let m = manifest();
    let mut offenders = Vec::new();
    for (section, deps) in dependency_tables(&m) {
        for (name, spec) in deps {
            if let Some(p) = spec.get("path").and_then(|v| v.as_str()) {
                if Path::new(p).is_absolute() || p.starts_with('~') {
                    offenders.push(format!("{section}.{name} -> {p}"));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "non-portable path dependencies: {offenders:?}"
    );
    assert!(
        !read("Cargo.toml").contains("\"/Users/"),
        "Cargo.toml mentions a /Users/ path"
    );
}

#[test]
fn git_dependencies_are_pinned_to_an_immutable_rev() {
    let m = manifest();
    let mut offenders = Vec::new();
    for (section, deps) in dependency_tables(&m) {
        for (name, spec) in deps {
            if spec.get("git").is_some() {
                let rev = spec.get("rev").and_then(|v| v.as_str()).unwrap_or("");
                let full_sha = rev.len() == 40 && rev.chars().all(|c| c.is_ascii_hexdigit());
                if !full_sha {
                    offenders.push(format!("{section}.{name} rev={rev:?}"));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "git deps must pin a full 40-hex rev: {offenders:?}"
    );
}

#[test]
fn removed_wasm4pm_algos_crate_is_not_referenced() {
    let m = manifest();
    for (section, deps) in dependency_tables(&m) {
        assert!(
            !deps.contains_key("wasm4pm-algos"),
            "{section} still declares wasm4pm-algos"
        );
    }
    let mut hits = Vec::new();
    for dir in ["src", "tests", "benches"] {
        for entry in walkdir::WalkDir::new(root().join(dir)) {
            let entry = entry.unwrap();
            if entry.path().extension().and_then(|e| e.to_str()) == Some("rs")
                && entry.path().file_name() != Some(std::ffi::OsStr::new("portable_build_guard.rs"))
            {
                let text = std::fs::read_to_string(entry.path()).unwrap();
                if text.contains("wasm4pm_algos") {
                    hits.push(entry.path().display().to_string());
                }
            }
        }
    }
    assert!(hits.is_empty(), "wasm4pm_algos referenced in: {hits:?}");
}

#[test]
fn lockfile_has_no_duplicate_package_identity() {
    let lock: toml::Value = read("Cargo.lock")
        .parse()
        .expect("Cargo.lock parses as TOML");
    let packages = lock
        .get("package")
        .and_then(|v| v.as_array())
        .expect("[[package]] entries");
    let mut seen: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for p in packages {
        let key = (
            p.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            p.get("version")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            p.get("source")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
        );
        *seen.entry(key).or_default() += 1;
    }
    let dups: Vec<_> = seen.into_iter().filter(|(_, n)| *n > 1).collect();
    assert!(
        dups.is_empty(),
        "duplicate (name, version, source) in Cargo.lock: {dups:?}"
    );
    assert!(
        !read("Cargo.lock").contains("/Users/"),
        "Cargo.lock records a workstation path"
    );
}

/// `(module name, preceding #[cfg]/#[path] attributes)` for every
/// `pub mod x;` / `mod x;` line in `src`.
fn declared_modules(src: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = src.lines().collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        let rest = t
            .strip_prefix("pub mod ")
            .or_else(|| t.strip_prefix("mod "));
        if let Some(rest) = rest {
            if let Some(name) = rest.strip_suffix(';') {
                let mut attrs = Vec::new();
                let mut j = i;
                while j > 0 && lines[j - 1].trim().starts_with("#[") {
                    j -= 1;
                    attrs.push(lines[j].trim().to_string());
                }
                attrs.reverse();
                out.push((name.trim().to_string(), attrs.join(" ")));
            }
        }
    }
    out
}

#[test]
fn lib_declares_each_module_once_per_cfg() {
    let mods = declared_modules(&read("src/lib.rs"));
    let mut seen = BTreeSet::new();
    let mut dups = Vec::new();
    for m in &mods {
        if !seen.insert(m.clone()) {
            dups.push(m.clone());
        }
    }
    assert!(
        dups.is_empty(),
        "duplicate module declarations in src/lib.rs: {dups:?}"
    );
}

#[test]
fn every_top_level_source_file_is_a_declared_module() {
    let mut declared: BTreeSet<String> = declared_modules(&read("src/lib.rs"))
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    declared.extend(
        declared_modules(&read("src/main.rs"))
            .into_iter()
            .map(|(n, _)| n),
    );
    // Crate roots, and the Windows socket file bound via `#[path]`.
    let exempt = ["lib", "main", "socket_windows"];
    let mut orphans = Vec::new();
    for entry in std::fs::read_dir(root().join("src")).unwrap() {
        let path = entry.unwrap().path();
        let name = if path.is_dir() {
            if !path.join("mod.rs").exists() {
                continue;
            }
            path.file_name().unwrap().to_string_lossy().to_string()
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            path.file_stem().unwrap().to_string_lossy().to_string()
        } else {
            continue;
        };
        if !exempt.contains(&name.as_str()) && !declared.contains(&name) {
            orphans.push(name);
        }
    }
    orphans.sort();
    assert!(
        orphans.is_empty(),
        "source files not declared as modules (dropped by a merge?): {orphans:?}"
    );
}

#[test]
fn toolchain_is_pinned_for_the_nightly_only_wasm4pm_compat() {
    let tc: toml::Value = read("rust-toolchain.toml")
        .parse()
        .expect("rust-toolchain.toml parses");
    let channel = tc
        .get("toolchain")
        .and_then(|t| t.get("channel"))
        .and_then(|c| c.as_str())
        .expect("toolchain.channel");
    let date = channel
        .strip_prefix("nightly-")
        .expect("wasm4pm-compat needs a dated nightly");
    let parts: Vec<&str> = date.split('-').collect();
    assert!(
        parts.len() == 3 && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())),
        "channel must be nightly-YYYY-MM-DD, got {channel}"
    );
}
