//! Swarm: 9 manufactured AtomVM nodes, one per wasm4pm cognition breed,
//! coordinated by the Hearsay-II blackboard breed.
//!
//! Each node is produced by the deterministic `manufacture()` pipeline
//! (so its AtomVM module compiles under real `erlc`, its Rust crate
//! compiles under real `cargo check`, its receipt is bound to the
//! upstream work order). Each node is driven by its assigned cognition
//! breed (ELIZA / CBR / DENDRAL / STRIPS / Prolog / MYCIN / GPS / SOAR
//! / Hearsay) against a shared `BreedInput` scenario. The 9 outputs are
//! then fused by the Hearsay-II breed acting as the consensus engine.
//!
//! This module is intentionally small — most heavy lifting is delegated
//! to `crate::manufacturing` (artifact generation) and
//! `wasm4pm_cognition::breeds::dispatch_breed_test` (cognition).

use crate::manufacturing::{manufacture, SolutionBundle, SolutionSpec};
use serde::{Deserialize, Serialize};
use wasm4pm_cognition::breeds::{dispatch_breed_test, BreedInput, BreedOutput};

/// The Hearsay-II breed identifier — the swarm's consensus engine.
/// Named separately so the fusion step can reference it without a bare
/// magic string, and so callers can check `breed == HEARSAY_BREED`
/// without duplicating the spelling.
pub const HEARSAY_BREED: &str = "hearsay";

pub const SWARM_BREEDS: &[&str] = &[
    "eliza", "cbr", "dendral", "strips", "prolog", "mycin", "gps", "soar", HEARSAY_BREED,
];

/// One node in the swarm: a breed name + its manufactured artifact bundle.
///
/// # Examples
///
/// ```
/// use open_ontologies::swarm::{manufacture_swarm, SwarmNode};
///
/// let nodes = manufacture_swarm("demo", &"c".repeat(64)).unwrap();
/// let node: &SwarmNode = &nodes[0];
/// // The breed field matches one of the nine SWARM_BREEDS entries.
/// assert!(!node.breed.is_empty());
/// // The bundle always contains at least one file.
/// assert!(!node.bundle.files.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct SwarmNode {
    pub breed: String,
    pub bundle: SolutionBundle,
}

/// One node's run result: input scenario + breed output.
///
/// # Examples
///
/// ```
/// use open_ontologies::swarm::NodeReport;
///
/// let report = NodeReport {
///     breed: "eliza".into(),
///     trace_steps: 3,
///     explanation: "Pattern matched on leakage fact.".into(),
///     selected: Some("centralized-revenue-engine".into()),
///     fact_count: 2,
/// };
/// assert_eq!(report.breed, "eliza");
/// assert_eq!(report.trace_steps, 3);
/// assert!(report.selected.is_some());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeReport {
    pub breed: String,
    pub trace_steps: usize,
    pub explanation: String,
    pub selected: Option<String>,
    pub fact_count: usize,
}

/// Swarm consensus produced by the Hearsay-II breed acting on the
/// 9 individual reports.
///
/// # Examples
///
/// ```
/// use open_ontologies::swarm::{SwarmConsensus, NodeReport};
///
/// let consensus = SwarmConsensus {
///     node_reports: vec![NodeReport {
///         breed: "hearsay".into(),
///         trace_steps: 5,
///         explanation: "Blackboard converged.".into(),
///         selected: Some("edge-distributed-reconciliation".into()),
///         fact_count: 4,
///     }],
///     consensus_explanation: "Hearsay-II majority vote resolved to edge architecture.".into(),
///     consensus_selected: Some("edge-distributed-reconciliation".into()),
///     consensus_trace_steps: 5,
/// };
/// assert_eq!(consensus.node_reports.len(), 1);
/// assert!(consensus.consensus_selected.is_some());
/// // SwarmConsensus is JSON-serializable.
/// let json = serde_json::to_string(&consensus).unwrap();
/// assert!(json.contains("consensus_explanation"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConsensus {
    pub node_reports: Vec<NodeReport>,
    pub consensus_explanation: String,
    pub consensus_selected: Option<String>,
    pub consensus_trace_steps: usize,
}

/// Build the SolutionSpec for one swarm node. Same `work_order_receipt
/// _hash` across the swarm — the swarm IS one work order, manufactured
/// into 9 nodes.
///
/// # Examples
///
/// ```
/// use open_ontologies::swarm::node_spec;
///
/// let hash = "a".repeat(64);
/// let spec = node_spec("myswarm", "eliza", &hash);
///
/// // The spec name is `{swarm_name}_{breed}`.
/// assert_eq!(spec.name, "myswarm_eliza");
/// // All nine nodes in a swarm share the same work-order receipt hash.
/// assert_eq!(spec.work_order_receipt_hash, hash);
/// // Infrastructure target is always AWS for the swarm.
/// assert_eq!(spec.iac_target, "aws");
/// // MCU target is always ESP-32 for the swarm.
/// assert_eq!(spec.mcu_target, "esp32");
/// ```
pub fn node_spec(swarm_name: &str, breed: &str, work_order_hash: &str) -> SolutionSpec {
    SolutionSpec {
        name: format!("{swarm_name}_{breed}"),
        description: format!(
            "Swarm node {breed} for {swarm_name}: AtomVM-resident cognition unit"
        ),
        iac_target: "aws".into(),
        region: "us-east-1".into(),
        supervisor_children: 4,
        mcu_target: "esp32".into(),
        work_order_receipt_hash: work_order_hash.to_string(),
    }
}

/// Manufacture all 9 nodes deterministically. Returns a vec of
/// (breed_name, bundle) tuples. Fails fast if any breed's spec
/// fails validation.
///
/// # Examples
///
/// ```
/// use open_ontologies::swarm::{manufacture_swarm, SWARM_BREEDS};
///
/// let hash = "b".repeat(64);
/// let nodes = manufacture_swarm("demo_swarm", &hash).expect("manufacture succeeded");
///
/// // The swarm always contains exactly nine nodes — one per breed.
/// assert_eq!(nodes.len(), 9);
/// // Every node carries a non-empty file bundle.
/// for node in &nodes {
///     assert!(!node.bundle.files.is_empty(), "{} has no files", node.breed);
///     // Each node's spec name encodes both the swarm name and the breed.
///     assert!(node.bundle.spec.name.contains(&node.breed));
///     // All nodes share the same upstream work-order receipt.
///     assert_eq!(node.bundle.spec.work_order_receipt_hash, hash);
/// }
/// // Every breed in SWARM_BREEDS is represented exactly once.
/// let mut breeds: Vec<&str> = nodes.iter().map(|n| n.breed.as_str()).collect();
/// breeds.sort_unstable();
/// let mut expected = SWARM_BREEDS.to_vec();
/// expected.sort_unstable();
/// assert_eq!(breeds, expected);
/// ```
pub fn manufacture_swarm(
    swarm_name: &str,
    work_order_hash: &str,
) -> Result<Vec<SwarmNode>, crate::defects::DefectClass> {
    let mut out = Vec::with_capacity(SWARM_BREEDS.len());
    for breed in SWARM_BREEDS {
        let spec = node_spec(swarm_name, breed, work_order_hash);
        let bundle = manufacture(&spec)?;
        out.push(SwarmNode {
            breed: (*breed).to_string(),
            bundle,
        });
    }
    Ok(out)
}

/// Run each node's assigned cognition breed against the shared
/// scenario. Returns a NodeReport per breed.
///
/// # Examples
///
/// ```no_run
/// use open_ontologies::swarm::{run_breeds, SWARM_BREEDS};
/// use wasm4pm_cognition::breeds::{BreedInput, Candidate};
///
/// let scenario = BreedInput {
///     intent: "architecture selection".into(),
///     candidates: vec![Candidate {
///         id: "option-a".into(),
///         score: 0.5,
///         eliminated: false,
///         elimination_reason: None,
///     }],
///     facts: vec![],
///     cases: vec![],
///     rules: vec![],
///     goals: vec![],
///     state: vec![],
/// };
/// let reports = run_breeds(&scenario);
/// // One output entry per breed, even if a breed abstained.
/// assert_eq!(reports.len(), SWARM_BREEDS.len());
/// for (breed, out) in &reports {
///     assert!(!out.explanation.is_empty(), "{breed} produced no explanation");
/// }
/// ```
pub fn run_breeds(scenario: &BreedInput) -> Vec<(String, BreedOutput)> {
    let mut out = Vec::with_capacity(SWARM_BREEDS.len());
    for breed in SWARM_BREEDS {
        // STRIPS / Prolog / MYCIN need preconditions that the canonical
        // scenario must already supply. We treat any breed failure as
        // a node-level abstention (empty BreedOutput) rather than
        // aborting the swarm — the consensus tolerates abstention.
        match dispatch_breed_test(breed, scenario) {
            Ok(o) => out.push(((*breed).to_string(), o)),
            Err(_e) => {
                // Synthesize an abstention BreedOutput so the breed is
                // accounted for in the consensus.
                let abst = BreedOutput {
                    breed: parse_breed_id(breed),
                    candidates: scenario.candidates.clone(),
                    facts: vec![],
                    selected: None,
                    explanation: format!("{breed}: abstained (preconditions not met)"),
                    inference_trace: vec![],
                };
                out.push(((*breed).to_string(), abst));
            }
        }
    }
    out
}

fn parse_breed_id(name: &str) -> wasm4pm_cognition::breeds::BreedId {
    use wasm4pm_cognition::breeds::BreedId;
    match name {
        "eliza" => BreedId::Eliza,
        "cbr" => BreedId::Cbr,
        "dendral" => BreedId::Dendral,
        "strips" => BreedId::Strips,
        "prolog" => BreedId::Prolog,
        "mycin" => BreedId::Mycin,
        "gps" => BreedId::Gps,
        "soar" => BreedId::Soar,
        _ => BreedId::Hearsay,
    }
}

/// Fuse the per-node reports via the Hearsay-II breed. The fusion
/// constructs a synthetic `BreedInput` whose `candidates` field
/// collects every breed's selected outcome (or its top scored
/// candidate) and runs Hearsay over them. Hearsay's blackboard
/// consensus model returns the multi-source winner.
///
/// # Examples
///
/// ```no_run
/// use open_ontologies::swarm::{fuse_via_hearsay, run_breeds};
/// use wasm4pm_cognition::breeds::{BreedInput, Candidate};
///
/// let scenario = BreedInput {
///     intent: "architecture selection".into(),
///     candidates: vec![Candidate {
///         id: "option-a".into(),
///         score: 0.5,
///         eliminated: false,
///         elimination_reason: None,
///     }],
///     facts: vec![],
///     cases: vec![],
///     rules: vec![],
///     goals: vec![],
///     state: vec![],
/// };
/// let reports = run_breeds(&scenario);
/// let consensus = fuse_via_hearsay(&scenario, &reports);
///
/// // Consensus aggregates all nine per-node reports.
/// assert_eq!(consensus.node_reports.len(), 9);
/// // The consensus carries a non-empty explanation from Hearsay-II.
/// assert!(!consensus.consensus_explanation.is_empty());
/// ```
pub fn fuse_via_hearsay(
    scenario: &BreedInput,
    reports: &[(String, BreedOutput)],
) -> SwarmConsensus {
    use wasm4pm_cognition::breeds::{Candidate, Fact};
    // Aggregate every breed's facts + selected into a synthetic input
    // for the Hearsay run. Each breed's `selected` becomes a candidate
    // (boosted by the breed's trace length, which is the breed's
    // "confidence" in the wasm4pm model).
    let mut hearsay_input = scenario.clone();
    let mut votes: std::collections::HashMap<String, (usize, Vec<String>)> =
        std::collections::HashMap::new();
    for (breed, out) in reports {
        if let Some(sel) = &out.selected {
            let entry = votes.entry(sel.clone()).or_insert((0, Vec::new()));
            entry.0 += out.inference_trace.len().max(1);
            entry.1.push(breed.clone());
        }
        for f in &out.facts {
            hearsay_input.facts.push(Fact {
                key: format!("breed_{}.{}", breed, f.key),
                value: f.value.clone(),
            });
        }
    }
    // Build candidates from the votes (so Hearsay has a non-empty
    // candidate set to consider). When no breed voted for anything,
    // fall back to the scenario's original candidates.
    if !votes.is_empty() {
        hearsay_input.candidates = votes
            .iter()
            .map(|(id, (weight, _voters))| Candidate {
                id: id.clone(),
                score: ((*weight) as f32) / 100.0,
                eliminated: false,
                elimination_reason: None,
            })
            .collect();
    }
    let hearsay_out = match dispatch_breed_test(HEARSAY_BREED, &hearsay_input) {
        Ok(o) => o,
        Err(_) => BreedOutput {
            breed: wasm4pm_cognition::breeds::BreedId::Hearsay,
            candidates: hearsay_input.candidates.clone(),
            facts: vec![],
            selected: None,
            explanation: format!("{HEARSAY_BREED}: abstained"),
            inference_trace: vec![],
        },
    };
    let node_reports: Vec<NodeReport> = reports
        .iter()
        .map(|(breed, out)| NodeReport {
            breed: breed.clone(),
            trace_steps: out.inference_trace.len(),
            explanation: out.explanation.clone(),
            selected: out.selected.clone(),
            fact_count: out.facts.len(),
        })
        .collect();
    SwarmConsensus {
        node_reports,
        consensus_explanation: hearsay_out.explanation,
        consensus_selected: hearsay_out.selected,
        consensus_trace_steps: hearsay_out.inference_trace.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_scenario() -> BreedInput {
        use wasm4pm_cognition::breeds::{Candidate, Case, Fact, Goal, Rule, StateAtom};
        BreedInput {
            intent: "RevOps revenue leakage detection across the booking pipeline".into(),
            candidates: vec![
                Candidate {
                    id: "centralized-revenue-engine".into(),
                    score: 0.5,
                    eliminated: false,
                    elimination_reason: None,
                },
                Candidate {
                    id: "edge-distributed-reconciliation".into(),
                    score: 0.5,
                    eliminated: false,
                    elimination_reason: None,
                },
            ],
            facts: vec![
                Fact { key: "scale".into(), value: "billion".into() },
                Fact { key: "leakage".into(), value: "detected".into() },
                Fact { key: "current".into(), value: "no-architecture".into() },
            ],
            cases: vec![Case {
                id: "case-001".into(),
                intent: "Reconciliation gap".into(),
                architecture: "centralized-revenue-engine".into(),
                outcome_score: 0.92,
                facts: vec![Fact { key: "scale".into(), value: "billion".into() }],
            }],
            rules: vec![
                Rule {
                    id: "r1".into(),
                    premise: vec!["scale=billion".into()],
                    conclusion: "favor=centralized-revenue-engine".into(),
                    certainty: 0.9,
                },
                Rule {
                    id: "establish".into(),
                    premise: vec!["current=no-architecture".into()],
                    conclusion: "performance=high".into(),
                    certainty: 1.0,
                },
            ],
            goals: vec![Goal {
                id: "g1".into(),
                predicate: "performance".into(),
                value: "high".into(),
            }],
            state: vec![StateAtom {
                predicate: "current".into(),
                value: "no-architecture".into(),
            }],
        }
    }

    #[test]
    fn manufacture_swarm_emits_nine_nodes() {
        let nodes = manufacture_swarm("test_swarm", &"a".repeat(64)).unwrap();
        assert_eq!(nodes.len(), 9);
        for n in &nodes {
            assert!(SWARM_BREEDS.contains(&n.breed.as_str()));
            assert!(!n.bundle.files.is_empty());
            // Every node has its breed name in the spec name
            assert!(n.bundle.spec.name.contains(&n.breed));
        }
    }

    #[test]
    fn run_breeds_returns_one_output_per_breed() {
        let reports = run_breeds(&fixture_scenario());
        assert_eq!(reports.len(), 9);
        for (breed, out) in &reports {
            // No breed should be entirely silent: at minimum it should
            // produce an explanation (real or abstention).
            assert!(
                !out.explanation.is_empty(),
                "{breed} produced no explanation"
            );
        }
    }

    #[test]
    fn fuse_via_hearsay_produces_consensus() {
        let scenario = fixture_scenario();
        let reports = run_breeds(&scenario);
        let consensus = fuse_via_hearsay(&scenario, &reports);
        assert_eq!(consensus.node_reports.len(), 9);
        // Consensus output is JSON-serializable.
        let json = serde_json::to_string(&consensus).expect("serialize consensus");
        assert!(json.contains("\"node_reports\""));
        assert!(json.contains("\"consensus_explanation\""));
    }

    #[test]
    fn swarm_node_specs_share_one_work_order_hash() {
        // The swarm IS one work order. All 9 nodes carry the same
        // upstream receipt — that is the swarm-binding contract.
        let hash = "b".repeat(64);
        let nodes = manufacture_swarm("contract_swarm", &hash).unwrap();
        for n in &nodes {
            assert_eq!(n.bundle.spec.work_order_receipt_hash, hash);
        }
    }
}
