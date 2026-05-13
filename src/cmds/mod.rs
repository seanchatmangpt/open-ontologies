//! Command Router Module — clap-noun-verb noun-verb auto-discovery
//!
//! Each sub-module groups related verbs under a noun.
//! clap-noun-verb discovers all `#[verb]` functions automatically.

pub mod helpers;
pub mod alignment;
pub mod clinical;
pub mod data;
pub mod doctor;
pub mod generated;
/// R10-1: Generated RevOps manufacturing stage constants. Do not edit directly.
/// Regenerate with: `ggen sync --manifest ggen-revops.toml`
pub mod generated_revops;
pub mod governance;
pub mod marketplace;
pub mod ontology;
pub mod server;
pub mod thesis;
