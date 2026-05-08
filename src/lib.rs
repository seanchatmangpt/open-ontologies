pub mod admission;
pub mod batch;
pub mod cache;
pub mod cell_ready;
pub mod defects;
pub mod error;
pub mod production_record;
pub mod receipts;
pub mod workflows;

pub use defects::{DefectClass, Deviation};
pub mod align;
pub mod config;
pub mod clinical;
pub mod drift;
pub mod enforce;
pub mod feedback;
pub mod graph;
pub mod ingest;
pub mod inputs;
pub mod lineage;
pub mod monitor;
pub mod ocel_store;
pub mod plan;
pub mod powl_bridge;
pub mod webhook;
pub mod mapping;
pub mod marketplace;
pub mod ontology;
pub mod reason;
pub mod registry;
pub mod repo;
pub mod runtime;
pub mod server;
pub mod shacl;
pub mod state;
pub mod schema;
pub mod socket;
pub mod sqlsource;
pub mod tableaux;
pub mod toolfilter;
#[cfg(feature = "embeddings")]
pub mod poincare;
#[cfg(feature = "embeddings")]
pub mod vecstore;
#[cfg(feature = "embeddings")]
pub mod embed;
#[cfg(feature = "embeddings")]
pub mod embed_remote;
#[cfg(feature = "embeddings")]
pub mod structembed;

// LLM Boundary Translator (Groq). Always available — reqwest+tokio are
// not feature-gated. The translator is a *proposer*, not an authority.
pub mod llm_translator;

// DSPy-style signature shapes — the language-to-contract boundary that
// molds LLM output before generation and gauges it after. Used by the
// shaped translator to constrain CTQ proposals to a specific output
// shape with retry-on-failure.
pub mod signature_shape;

// Solution Manufacturing — Phase 4. Multi-target deterministic generator
// for IaC + Rust + Erlang + AtomVM, gated by SolutionManufactured
// admission op.
pub mod manufacturing;

// Swarm — manufactures 9 AtomVM cognition nodes (one per wasm4pm
// breed), runs each breed against a shared scenario, fuses outputs
// via Hearsay-II consensus.
pub mod swarm;
