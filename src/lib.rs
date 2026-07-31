//! Open Ontologies library surface.
//!
//! Every module is declared exactly once. Feature-gated modules remain explicit,
//! and model-facing surfaces are proposers rather than execution authorities.

pub mod a2a;
pub mod admission;
pub mod align;
pub mod align_fuzzy;
pub mod attestation;
pub mod batch;
pub mod borderline_loop;
pub mod bootstrap;
pub mod cache;
pub mod cell8;
pub mod cell_ready;
pub mod claimcheck;
pub mod civex;
#[cfg(feature = "causal-pywhy")]
pub mod civex_pywhy;
pub mod classify_el;
pub mod clinical;
pub mod coevolve;
pub mod config;
pub mod cq;
pub mod defects;
pub mod drift;
pub mod enforce;
pub mod error;
pub mod feedback;
pub mod graph;
pub mod guide;
pub mod health_guardian;
pub mod ingest;
pub mod inputs;
pub mod kgcl;
pub mod language;
pub mod lineage;
pub mod llm_input;
pub mod llm_translator;
pub mod manufacturing;
pub mod mapping;
pub mod marketplace;
pub mod mcpp_gate;
pub mod monitor;
pub mod ocel_store;
pub mod ontology;
pub mod plan;
pub mod plan_classical;
pub mod plan_pddl;
pub mod plan_validate;
pub mod powl_bridge;
pub mod production_record;
pub mod projection_check;
pub mod reason;
pub mod receipt_archive;
pub mod receipt_chain;
pub mod receipts;
pub mod registry;
pub mod repo;
pub mod retention;
pub mod runtime;
pub mod schema;
pub mod segment_retrieve;
pub mod server;
pub mod shacl;
pub mod signature_shape;
#[cfg(unix)]
pub mod socket;
#[cfg(windows)]
#[path = "socket_windows.rs"]
pub mod socket;
pub mod sql_sync;
pub mod sqlsource;
pub mod state;
pub mod subprocess;
pub mod swarm;
pub mod tableaux;
pub mod telemetry;
pub mod tenant;
pub mod toolfilter;
pub mod verifier_worker;
pub mod verify;
pub mod vocab_check;
pub mod webhook;
pub mod workflows;

#[cfg(feature = "embeddings")]
pub mod embed;
#[cfg(feature = "embeddings")]
pub mod embed_remote;
#[cfg(feature = "embeddings")]
pub mod hnsw_index;
#[cfg(feature = "embeddings")]
pub mod poincare;
#[cfg(feature = "embeddings")]
pub mod structembed;
#[cfg(feature = "embeddings")]
pub mod vecstore;

pub use defects::{DefectClass, Deviation};
