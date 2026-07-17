//! Command Router Module — clap-noun-verb noun-verb auto-discovery
//!
//! Each sub-module groups related verbs under a noun.
//! clap-noun-verb discovers all `#[verb]` functions automatically.
//!
//! # CLI structure
//!
//! ```no_run
//! // Noun-verb dispatch — each module is a noun, each #[verb] function is a verb:
//! //
//! //   open-ontologies ontology load my-onto.ttl
//! //   open-ontologies ontology query --sparql "SELECT ?s WHERE { ?s a owl:Class }"
//! //   open-ontologies marketplace list
//! //   open-ontologies marketplace install foaf
//! //   open-ontologies clinical crosswalk --code J18.9 --system ICD-10
//! //   open-ontologies alignment align source.ttl --target target.ttl
//! //   open-ontologies doctor full
//! ```

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
