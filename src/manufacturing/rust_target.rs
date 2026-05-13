//! Rust target generator — service crate skeleton.
//!
//! Emits `rust/Cargo.toml`, `rust/src/lib.rs`, `rust/src/main.rs`. The
//! service exposes a single `manufactured_solution_name()` function so
//! downstream Rust code can verify it's calling the right manufactured
//! crate.

use super::{with_header, ManufacturedFile, SolutionSpec};

pub fn generate(spec: &SolutionSpec) -> Vec<ManufacturedFile> {
    vec![
        file("rust/Cargo.toml", &generate_cargo_toml(spec), spec),
        file("rust/src/lib.rs", &generate_lib_rs(spec), spec),
        file("rust/src/main.rs", &generate_main_rs(spec), spec),
    ]
}

fn file(path: &str, body: &str, spec: &SolutionSpec) -> ManufacturedFile {
    ManufacturedFile {
        path: path.to_string(),
        contents: with_header(spec, path, body),
        target: "rust".to_string(),
    }
}

fn generate_cargo_toml(spec: &SolutionSpec) -> String {
    // Cargo.toml uses `#` for comments — receipt header lives in `#`
    // form there, but for consistency we use the same with_header()
    // entry point. The default comment_prefix_for() returns `#` for
    // unknown extensions, but we override here by passing a path
    // ending in `.toml` which falls through to `#`. The existing
    // helper handles `.rs` and `.tf` (`//`), `.erl`/`.hrl` (`%%`),
    // everything else `#`.
    format!(
        "[package]\n\
         name = \"{name}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2021\"\n\
         description = \"{desc}\"\n\
         \n\
         [lib]\n\
         path = \"src/lib.rs\"\n\
         \n\
         [[bin]]\n\
         name = \"{name}\"\n\
         path = \"src/main.rs\"\n\
         \n\
         [dependencies]\n\
         serde = {{ version = \"1\", features = [\"derive\"] }}\n\
         serde_json = \"1\"\n\
         tokio = {{ version = \"1\", features = [\"full\"] }}\n",
        name = spec.name,
        desc = spec.description.replace('"', "'"),
    )
}

fn generate_lib_rs(spec: &SolutionSpec) -> String {
    format!(
        "//! {desc}\n\
         //!\n\
         //! Manufactured by OntoStar Solution Manufacturing pipeline.\n\
         //! Bound to work-order receipt: {wor}\n\
         \n\
         /// Returns the solution name this crate was manufactured for.\n\
         /// External verifiers can call this to confirm the binary was\n\
         /// produced from the expected SolutionSpec.\n\
         pub fn manufactured_solution_name() -> &'static str {{\n\
         \x20\x20\x20\x20\"{name}\"\n\
         }}\n\
         \n\
         /// Returns the upstream work-order receipt hash this stack is\n\
         /// bound to. Embedded as a const so it's part of the binary.\n\
         pub const WORK_ORDER_RECEIPT: &str = \"{wor}\";\n\
         \n\
         #[cfg(test)]\n\
         mod tests {{\n\
         \x20\x20\x20\x20use super::*;\n\
         \x20\x20\x20\x20#[test]\n\
         \x20\x20\x20\x20fn solution_name_matches() {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20assert_eq!(manufactured_solution_name(), \"{name}\");\n\
         \x20\x20\x20\x20}}\n\
         \x20\x20\x20\x20#[test]\n\
         \x20\x20\x20\x20fn work_order_receipt_is_64_hex() {{\n\
         \x20\x20\x20\x20\x20\x20\x20\x20assert_eq!(WORK_ORDER_RECEIPT.len(), 64);\n\
         \x20\x20\x20\x20\x20\x20\x20\x20assert!(WORK_ORDER_RECEIPT.chars().all(|c| c.is_ascii_hexdigit()));\n\
         \x20\x20\x20\x20}}\n\
         }}\n",
        desc = spec.description,
        name = spec.name,
        wor = spec.work_order_receipt_hash,
    )
}

fn generate_main_rs(spec: &SolutionSpec) -> String {
    format!(
        "//! {name} — manufactured service binary.\n\
         \n\
         use {name}::{{manufactured_solution_name, WORK_ORDER_RECEIPT}};\n\
         \n\
         #[tokio::main]\n\
         async fn main() {{\n\
         \x20\x20\x20\x20println!(\"{{}} (work order: {{}})\", manufactured_solution_name(), WORK_ORDER_RECEIPT);\n\
         }}\n",
        name = spec.name,
    )
}
