pub mod a2a;
pub mod actuation;
pub mod admission;
pub mod attestation;
pub mod autoreceipt_law;
pub mod care_followup;
pub mod ocel_manufacturer;
pub mod batch;
pub mod bootstrap;
pub mod cache;
pub mod cell8;
pub mod cell_ready;
pub mod defects;
pub mod error;
pub mod production_record;
pub mod receipts;
pub mod workflows;

pub use defects::{DefectClass, Deviation};

pub mod align;
pub mod align_fuzzy;
pub mod borderline_loop;
pub mod civex;
#[cfg(feature = "causal-pywhy")]
pub mod civex_pywhy;
pub mod classify_el;
pub mod clinical;
pub mod coevolve;
pub mod config;
pub mod cq;
pub mod drift;
pub mod dynamics;
pub mod dynamics_bcplus;
pub mod eval_alignment;
pub mod eval_rag;
pub mod extract_scaffold;
pub mod flora_pipeline;
pub mod policy;
pub mod projection_check;
pub mod shape_combinatorics;

pub mod enforce;
pub mod feedback;
pub mod ggen_bridge;
pub mod graph;
pub mod ingest;
pub mod inputs;
pub mod lineage;
pub mod monitor;
pub mod ocel_store;
pub mod plan;
pub mod powl_bridge;
pub mod mapping;
pub mod marketplace;
pub mod ontology;
pub mod thesis_doctor;
pub mod reason;
pub mod registry;
pub mod retention;
pub mod receipt_archive;
pub mod receipt_chain;
pub mod repo;
pub mod runtime;
pub mod guide;
pub mod ghf;
pub mod mcpp_gate;
pub mod server;
pub mod shacl;
pub mod state;
pub mod tenant;
pub mod schema;

#[cfg(unix)]
pub mod socket;
#[cfg(windows)]
#[path = "socket_windows.rs"]
pub mod socket;

pub mod sql_sync;
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
pub mod hnsw_index;
#[cfg(feature = "embeddings")]
pub mod structembed;

pub mod kgcl;
pub mod language;
pub mod plan_classical;
pub mod plan_pddl;
pub mod plan_validate;
pub mod segment_retrieve;
pub mod webhook;

pub mod llm_input;
pub mod llm_translator;
pub mod subprocess;
pub mod signature_shape;
pub mod manufacturing;
pub mod swarm;
pub mod verify;
pub mod verifier_worker;
pub mod telemetry;
pub mod health_guardian;
pub mod autoreceipt;
