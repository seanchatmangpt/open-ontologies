//! Shared integration-test helpers (R4 WA, §24).
//!
//! Sub-modules in `tests/` are per-binary, so each consumer declares its
//! own `mod common;` linkage and Cargo treats the sibling `common/`
//! directory as the module body. This file is the module root; submodules
//! are re-exported below.
//!
//! Some helpers may be unused by certain test binaries; that is expected
//! and the `dead_code` allow on each submodule is intentional.

pub mod sparql_capture;
