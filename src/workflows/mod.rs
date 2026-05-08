//! OntoStar workflow scope + built-in catalog.
//!
//! Stream 1: workflow declaration / closure and the static catalog of POWL
//! standard work shipped with OntoStar. The actual POWL parsing/conformance
//! checking lives in Stream 2 (wasm4pm bridge).

pub mod builtin;
pub mod scope;

pub use builtin::{by_name, BuiltinWorkflow, BUILTIN_WORKFLOWS};
pub use scope::WorkflowScope;
