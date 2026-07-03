//! CICD Commands — release standing and compliance checks

use clap_noun_verb::Result as NounVerbResult;
use clap_noun_verb_macros::verb;

use super::helpers::to_verb_err;
use open_ontologies::autoreceipt::{
    VerifyOutput, DoctorOutput, ExplainOutput, FixOutput, ReleaseOutput, TaskOutput,
    run_verify, run_doctor, run_explain, run_fix, run_release, run_task,
};

// --- Verbs ---

/// Checks release standing against the rules.
/// Checks expected/observed OCEL files and boundary outputs.
#[verb]
fn verify(data_dir: Option<String>) -> NounVerbResult<VerifyOutput> {
    run_verify(data_dir).map_err(to_verb_err)
}

/// Run health checks for cargo-cicd environment.
#[verb]
fn doctor(data_dir: Option<String>) -> NounVerbResult<DoctorOutput> {
    run_doctor(data_dir).map_err(to_verb_err)
}

/// Trace release law rules and provide explanation.
#[verb]
fn explain(rule_id: String) -> NounVerbResult<ExplainOutput> {
    run_explain(rule_id).map_err(to_verb_err)
}

/// Automatically attempt to fix common conformance violations.
#[verb]
fn fix(rule_id: Option<String>) -> NounVerbResult<FixOutput> {
    run_fix(rule_id).map_err(to_verb_err)
}

/// Promotes the standing state to ADMITTED and signs the release receipt.
#[verb]
fn release(version: String) -> NounVerbResult<ReleaseOutput> {
    run_release(version).map_err(to_verb_err)
}

/// Lists all outstanding tasks required to satisfy the release law.
#[verb]
fn task() -> NounVerbResult<TaskOutput> {
    run_task().map_err(to_verb_err)
}

