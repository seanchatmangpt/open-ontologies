//! OntoStar workflow scope + built-in catalog.
//!
//! Stream 1: workflow declaration / closure and the static catalog of POWL
//! standard work shipped with OntoStar. The actual POWL parsing/conformance
//! checking lives in Stream 2 (wasm4pm bridge).
//!
//! # Examples
//!
//! Re-exports from `builtin` are reachable via the module path:
//!
//! ```
//! use open_ontologies::workflows::{BUILTIN_WORKFLOWS, by_name, BuiltinWorkflow};
//!
//! // The catalog is non-empty.
//! assert!(!BUILTIN_WORKFLOWS.is_empty());
//!
//! // Re-exported `by_name` resolves the same as `builtin::by_name`.
//! let w: &BuiltinWorkflow = by_name("Alignment").expect("Alignment is in catalog");
//! assert_eq!(w.name, "Alignment");
//! ```
//!
//! All POWL strings in the catalog are non-trivial (longer than 10 chars),
//! confirming they are not placeholder stubs:
//!
//! ```
//! use open_ontologies::workflows::BUILTIN_WORKFLOWS;
//!
//! for w in BUILTIN_WORKFLOWS {
//!     assert!(
//!         w.powl_string.len() > 10,
//!         "suspiciously short POWL string for workflow '{}': {:?}",
//!         w.name,
//!         w.powl_string,
//!     );
//! }
//! ```

pub mod builtin;
pub mod scope;

pub use builtin::{by_name, BuiltinWorkflow, BUILTIN_WORKFLOWS};
pub use scope::WorkflowScope;
