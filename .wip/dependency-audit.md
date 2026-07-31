# Dependency boundary audit

- exact commit: `6ff86d3712cc88b3675f9115b0ea59862756699d`
- exact tree: `ef749a625b62435c03da71311e5429ec4f9e6b9d`
- standing: **PARTIAL_ALIVE**
- absolute path dependencies: **0**
- duplicate lock package keys: **0**
- dependency use sites: **101**

## Cargo metadata

Exit: `0`

```text
info: syncing channel updates for nightly-x86_64-unknown-linux-gnu
info: latest update on 2026-07-31 for version 1.99.0-nightly (8ab9fdff5 2026-07-30)
info: downloading 3 components
```

## Absolute path dependencies


## Duplicate lock package keys


## Dependency use sites

- `.github/workflows/wip-dispatch.yml:330` — `s = s.replace(') -> anyhow::Result<mcpp_core::proof_writer::OcelEvidence> {', ') -> anyhow::Result<Vec<u8>> {')`
- `.github/workflows/wip-dispatch.yml:331` — `s = s.replace('    Ok(mcpp_core::proof_writer::OcelEvidence::from_bytes(ocel_json))', '    Ok(ocel_json)')`
- `.github/workflows/wip-dispatch.yml:332` — `if 'mcpp_core::' in s:`
- `.github/workflows/wip-dispatch.yml:333` — `raise SystemExit('unresolved mcpp_core reference remains')`
- `benches/swarm_bench.rs:10` — `use wasm4pm_cognition::breeds::{BreedInput, Candidate, Case, Fact, Goal, Rule, StateAtom};`
- `docs/01-architecture.md:21` — `│  src/swarm.rs · wasm4pm_cognition::breeds                       │`
- `docs/02-quickstart.md:70` — `Expected output: a candidate POWL string molded by a `SignatureShape`, then validated by `wasm4pm::parse`. If the LLM emits invalid POWL, the refine loop retries with a typed `ValidationFailure` until the shape gauges pass or the budget is exhausted. See `tests/real_groq_powl.rs`.`
- `docs/06-llm-boundary.md:45` — ``tests/real_groq_powl.rs` replicates the pm4py paper's example end-to-end: a natural-language process description, a `SignatureShape` defining the POWL output field, a real Groq call (no mocks, no replay), then `wasm4pm::parse` as the structural gauge. Invalid POWL → typed `ValidationFailure` → refine loop. Five LLM boundaries got the same treatment in commits `1b7d6cc` and `619c3b1`:`
- `docs/research/chapter3/01_powl_bridge_analysis.md:6` — `The bridge acts as an adapter. It parses the declarative POWL string into a `PowlArena` using `wasm4pm::powl_parser::parse_powl_model_string`. It then converts the POWL AST into a Petri Net (`wasm4pm::powl::conversion::to_petri_net::apply`).`
- `docs/research/chapter3/01_powl_bridge_analysis.md:8` — `When evaluating a trace, the bridge projects the OCEL log into a flat string slice and calls `wasm4pm::powl::conformance::token_replay::replay_trace`.`
- `emitted/open-ontologies-audit.md:134` — `- `wasm4pm_types_stub` — OCEL 2.0 Event/Trace/EventLog types`
- `emitted/open-ontologies-audit.md:135` — `- `wasm4pm_cognition_stub` — 9-breed symbolic AI dispatch (Eliza, CBR, Dendral, STRIPS, Prolog, MYCIN, GPS, Soar, Hearsay)`
- `emitted/open-ontologies-audit.md:136` — `- `wasm4pm_algos_stub` — Token replay conformance, Alpha algorithm discovery`
- `ggen.toml:297` — `{ name = "wasm4pm-algos-stub", query = { inline = "SELECT (\"check_conformance_alignment|token_replay|alignment_stats|activity_coverage\" AS ?itemNames) (\"fn check_conformance_alignment(trace: &Trace, net: &PowlPetriNet) -> Result<f64, String>|fn token_replay(log: &EventLog, net: &PowlPetriNet) -> Result<TraceReplayResult, String>|fn alignment_stats(alignments: &[Alignment]) -> AlignmentStats|fn activity_coverage(log: &EventLog, net: &PowlPetriNet) -> f64\" AS ?methodSignatures) (\"Result<f64, `
- `ggen.toml:298` — `{ name = "wasm4pm-cognition-stub", query = { inline = "SELECT (\"breed_eliza|breed_cbr|breed_dendral|breed_strips|breed_prolog|breed_mycin|breed_gps|breed_soar|breed_hearsay\" AS ?itemNames) (\"fn breed_eliza(input: &BreedInput) -> Result<BreedOutput, String>|fn breed_cbr(input: &BreedInput) -> Result<BreedOutput, String>|fn breed_dendral(input: &BreedInput) -> Result<BreedOutput, String>|fn breed_strips(input: &BreedInput) -> Result<BreedOutput, String>|fn breed_prolog(input: &BreedInput) -> Re`
- `ggen.toml:300` — `{ name = "wasm4pm-types-stub", query = { file = ".specify/queries/wasm4pm-types-stub.rq" }, template = { file = ".specify/templates/wasm4pm-types-stub.rs.tera" }, output_file = "src/wasm4pm_types_stub.rs", mode = "Overwrite" },`
- `src/cmds/server.rs:311` — `#[cfg(feature = "mcpp")]`
- `src/cmds/server.rs:312` — `let server: MaybeGatedServer = match mcpp_core::receipt::KeyLoader::from_env() {`
- `src/cmds/server.rs:323` — `#[cfg(not(feature = "mcpp"))]`
- `src/cmds/server.rs:471` — `#[cfg(feature = "mcpp")]`
- `src/cmds/server.rs:473` — `mcpp_core::receipt::KeyLoader::from_env().unwrap_or(None);`
- `src/cmds/server.rs:474` — `#[cfg(feature = "mcpp")]`
- `src/cmds/server.rs:505` — `#[cfg(feature = "mcpp")]`
- `src/feedback/discovery.rs:9` — `//! Adapter note: the plan calls for `wasm4pm::powl::discovery::choice_graph::discover_choice_graph``
- `src/feedback/discovery.rs:26` — `use wasm4pm_types::event_log::{Attribute, AttributeValue};`
- `src/feedback/discovery.rs:27` — `use wasm4pm_types::{Event, EventLog, Trace};`
- `src/feedback/discovery.rs:130` — `let wasm4pm_log = wasm4pm::models::EventLog::from(log);`
- `src/feedback/discovery.rs:134` — `wasm4pm_types::admission::Admission::<_, ()>::new(wasm4pm_log.clone()).into_evidence();`
- `src/feedback/discovery.rs:135` — `let petri = match wasm4pm::algorithms::discover_alpha_plus_plus_from_log(`
- `src/feedback/discovery.rs:145` — `let conf = wasm4pm::conformance::token_replay_pure(&wasm4pm_log, &petri, "concept:name");`
- `src/inputs.rs:1719` — `/// `wasm4pm_cognition::breeds::BreedInput` JSON. Must contain at`
- `src/lineage.rs:3` — `use wasm4pm_types::event_log::{Attribute, AttributeValue, Attributes};`
- `src/lineage.rs:4` — `use wasm4pm_types::{Event, EventLog, Trace};`
- `src/lineage.rs:606` — `///             && matches!(&a.value, wasm4pm_types::event_log::AttributeValue::String(v) if v == "s1")`
- `src/lineage.rs:618` — `///     wasm4pm_types::event_log::AttributeValue::String(v) if v == "G:admission_granted"`
- `src/lineage.rs:665` — `///     wasm4pm_types::event_log::AttributeValue::String(v) if v == "complete"`
- `src/mcpp_gate.rs:31` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:34` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:80` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:126` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:136` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:144` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:156` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:168` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:180` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:192` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:204` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:212` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:219` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:229` — `/// # #[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:253` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:347` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:359` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:385` — `#[cfg(feature = "mcpp")]`
- `src/mcpp_gate.rs:403` — `#[cfg(feature = "mcpp")]`
- `src/ocel_store.rs:5` — `use wasm4pm_types::ocel::{OCEL, OCELObject};`
- `src/ocel_store.rs:467` — `/// `wasm4pm::powl::conformance::token_replay` via the bridge. Persists a`
- `src/ocel_store.rs:1017` — `use wasm4pm_types::ocel::OCELType;`
- `src/ocel_store.rs:1027` — `use wasm4pm_types::ocel::OCELType;`
- `src/ocel_store.rs:1037` — `use wasm4pm_types::ocel::{`
- `src/powl_bridge.rs:14` — `//! - `wasm4pm::powl_parser::parse_powl_model_string(s, &mut PowlArena) -> Result<u32, String>``
- `src/powl_bridge.rs:15` — `//! - `wasm4pm::powl::conversion::to_petri_net::apply(&PowlArena, root) -> PowlPetriNetResult``
- `src/powl_bridge.rs:16` — `//! - `wasm4pm::powl::conformance::token_replay::replay_trace(&PetriNet, &Marking, &Marking, &Trace) -> TraceReplayResult``
- `src/powl_bridge.rs:17` — `//! - `wasm4pm::powl::conformance::token_replay::compute_fitness(&PetriNet, &Marking, &Marking, &EventLog) -> FitnessResult``
- `src/powl_bridge.rs:40` — `pub use wasm4pm::powl::conformance::token_replay::{FitnessResult, TraceReplayResult};`
- `src/powl_bridge.rs:41` — `pub use wasm4pm::powl_arena::PowlArena;`
- `src/powl_bridge.rs:42` — `pub use wasm4pm::powl_event_log::{Event, EventLog, Trace};`
- `src/powl_bridge.rs:43` — `pub use wasm4pm::powl_models::{PowlMarking, PowlPetriNet};`
- `src/powl_bridge.rs:97` — `/// `wasm4pm::powl_parser::parse_powl_model_string`.`
- `src/powl_bridge.rs:123` — `let root = wasm4pm::powl_parser::parse_powl_model_string(powl_string, &mut self.arena)`
- `src/powl_bridge.rs:126` — `let pn = wasm4pm::powl::conversion::to_petri_net::apply(&self.arena, root);`
- `src/powl_bridge.rs:145` — `Ok(wasm4pm::powl::conformance::token_replay::replay_trace(`
- `src/powl_bridge.rs:154` — `/// `wasm4pm::powl::conformance::token_replay::compute_fitness`.`
- `src/powl_bridge.rs:160` — `Ok(wasm4pm::powl::conformance::token_replay::compute_fitness(`
- `src/server.rs:8454` — `let breed_input: wasm4pm_cognition::breeds::BreedInput = match serde_json::from_str(`
- `src/server.rs:8466` — `let dispatched = wasm4pm_cognition::breeds::dispatch_breed_test(&breed, &breed_input);`
- `src/swarm.rs:14` — `//! `wasm4pm_cognition::breeds::dispatch_breed_test` (cognition).`
- `src/swarm.rs:18` — `use wasm4pm_cognition::breeds::{BreedInput, BreedOutput, dispatch_breed_test};`
- `src/swarm.rs:288` — `/// use wasm4pm_cognition::breeds::{BreedInput, Candidate};`
- `src/swarm.rs:340` — `fn parse_breed_id(name: &str) -> wasm4pm_cognition::breeds::BreedId {`
- `src/swarm.rs:341` — `use wasm4pm_cognition::breeds::BreedId;`
- `src/swarm.rs:365` — `/// use wasm4pm_cognition::breeds::{BreedInput, Candidate};`
- `src/swarm.rs:393` — `use wasm4pm_cognition::breeds::{Candidate, Fact};`
- `src/swarm.rs:431` — `breed: wasm4pm_cognition::breeds::BreedId::Hearsay,`
- `src/swarm.rs:464` — `use wasm4pm_cognition::breeds::{Candidate, Case, Fact, Goal, Rule, StateAtom};`
- `src/wasm4pm_algos_stub.rs:13` — `//!   Tier 3  — src/wasm4pm_algos_stub.rs       (this file, wired to lib)`
- `src/wasm4pm_algos_stub.rs:20` — `// PETRI NET MODELS (from wasm4pm::powl_models)`
- `src/wasm4pm_cognition_stub.rs:10` — `//!   Tier 3  — src/wasm4pm_cognition_stub.rs (this file, wired to wasm4pm crate)`
- `src/wasm4pm_cognition_stub.rs:252` — `Err("wasm4pm_cognition not yet implemented: dispatch_breed_test unavailable".to_string())`
- `tests/cell_ready_fixtures/mod.rs:24` — `use wasm4pm_cognition::breeds::{BreedInput, Candidate, Case, Fact, Goal, Rule, StateAtom};`
- `tests/hearsay_returns_typed_consensus.rs:22` — `use wasm4pm_cognition::breeds::{BreedInput, BreedOutput};`
- `tests/mcpp_gate_smoke.rs:45` — `#[cfg(feature = "mcpp")]`
- `tests/mcpp_gate_smoke.rs:61` — `#[cfg(feature = "mcpp")]`
- `tests/mcpp_gate_smoke.rs:92` — `#[cfg(feature = "mcpp")]`
- `tests/old_ai_station_dispatch.rs:3` — `//! Drives `wasm4pm_cognition::breeds::dispatch_breed_test` against all 9`
- `tests/old_ai_station_dispatch.rs:16` — `use wasm4pm_cognition::breeds::{`
- `tests/powl_bridge.rs:4` — `//! `wasm4pm::powl::conformance::token_replay`. The bridge is plumbing only.`
- `tests/real_swarm_e2e.rs:7` — `use wasm4pm_cognition::breeds::{BreedInput, Candidate, Case, Fact, Goal, Rule, StateAtom};`
- `tests/revops_old_ai_stations.rs:21` — `use wasm4pm_cognition::breeds::{`
- `tests/saboteur_meta.rs:15` — `//! `open_ontologies` or `wasm4pm_cognition` crates.`
- `tests/saboteur_meta.rs:96` — `use wasm4pm_cognition::breeds::{BreedId, BreedOutput, Candidate};`
