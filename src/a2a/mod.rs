//! T2-1 Agent-to-Agent (A2A) protocol support.
//!
//! Implements the A2A protocol using the `a2a-rs` library for inter-agent
//! communication. This module provides:
//! - Agent card discovery (/.well-known/agent.json)
//! - Async message handling via AsyncMessageHandler
//! - Task storage backed by StateDb
//! - HTTP router mounting on `/a2a` endpoint

pub mod agent_card;
pub mod handler;
pub mod task_store;
pub mod router;

pub use router::build_a2a_router;
