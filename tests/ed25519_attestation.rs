//! Real Ed25519 attestation tests for the Cell8 A10 path.
//!
//! Replaces the digest-equality tautology stub with cryptographic
//! verification. Five tests:
//!
//!   1. `signed_receipt_admits_under_real_verify_strict` — happy path.
//!   2. `unsigned_receipt_passes_only_when_legacy_allowed` — backwards
//!      compat path: `signature: None` + `allow_legacy_unsigned: true`.
//!   3. `unsigned_receipt_denied_when_legacy_disabled` — production
//!      default: `signature: None` + `allow_legacy_unsigned: false` →
//!      AttestationMissing.
//!   4. `unknown_signing_key_yields_attestation_invalid` — sig present
//!      but the verifier has no matching public key.
//!   5. `receipt_replay_attack_rejected` — round-2 cascade attack: take
//!      a valid signature from receipt A and paste it onto receipt B
//!      (different artifact_hash). Must fail with
//!      `AttestationInvalid { reason: "signature_invalid" }`.

use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
use ed25519_dalek::pkcs8::EncodePublicKey;
use ed25519_dalek::SigningKey;
use open_ontologies::attestation::{Signer, TrustedKeys};
use open_ontologies::cell_ready::{cell_ready, CellReadyInputs, PowlOpRef};
use open_ontologies::defects::DefectClass;
use open_ontologies::ocel_store::OcelStore;
use open_ontologies::production_record::ProductionRecord;
use open_ontologies::state::StateDb;
use open_ontologies::workflows::WorkflowScope;
use rand_core::OsRng;
use tempfile::tempdir;

const HEX32: &str = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
const HEX32B: &str = "1111111111111111111111111111111111111111111111111111111111111111";

fn fresh_db() -> StateDb {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ed25519-attestation-test.db");
    std::mem::forget(dir);
    StateDb::open(&path).expect("open StateDb")
}

fn setup_scope(db: &StateDb, session: &str) -> String {
    let scope = WorkflowScope::new(db, session);
    let token = scope
        .open(None, Some("PO=(nodes={a, b}, order={a-->b})"), None)
        .expect("open scope");
    scope.close(&token).expect("close scope");
    let conn = db.conn();
    conn.execute(
        "INSERT INTO conformance_runs (
             run_id, scope_token, fitness, precision,
             generalization, simplicity, verdict, defects_json,
             trace_canonical_hash, ran_at
         ) VALUES (?1, ?2, 0.99, 0.99, NULL, NULL, 'conform', '[]', ?3, ?4)",
        rusqlite::params![
            format!("run-{}", token),
            &token,
            HEX32,
            chrono::Utc::now().to_rfc3339(),
        ],
    )
    .expect("insert conformance row");
    token
}

fn make_signer_and_trust() -> (Signer, TrustedKeys) {
    let sk = SigningKey::generate(&mut OsRng);
    let signer = Signer::from_bytes(&sk.to_bytes());
    let mut trust = TrustedKeys::new();
    let pem = sk
        .verifying_key()
        .to_public_key_pem(LineEnding::LF)
        .expect("encode pub pem");
    trust.insert_pem(&pem).expect("insert pem");
    (signer, trust)
}

/// Build a "preview" production record for the given scope/session/artifact-hash
/// matching the one cell_ready will reconstruct internally for verification.
fn preview_record(
    scope_token: &str,
    artifact_hash_hex: &str,
    powl_string: &str,
) -> ProductionRecord {
    let powl_hash = *blake3::hash(powl_string.as_bytes()).as_bytes();
    fn parse(s: &str) -> [u8; 32] {
        let mut out = [0u8; 32];
        for i in 0..32 {
            out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap();
        }
        out
    }
    ProductionRecord {
        artifact_hash: parse(artifact_hash_hex),
        scope_token: scope_token.to_string(),
        declared_powl_hash: powl_hash,
        ocel_canonical_hash: parse(HEX32),
        conformance_run_id: "run-test".to_string(),
        gate_config_hash: parse(HEX32),
        production_law_version: "ontostar-1.0.0".to_string(),
        defects_taxonomy_version:
            open_ontologies::defects::DEFECTS_TAXONOMY_VERSION.to_string(),
        gates_passed: vec![
            "A1_WorkflowDeclared".into(),
            "A2_ScopeClosed".into(),
            "A3_OCELComplete".into(),
            "A4_POWLReplayPass".into(),
            "A5_ThresholdPass".into(),
            "A6_RequiredStagesPresent".into(),
            "A7_NoBypassRevocation".into(),
            "A8_ReceiptValid".into(),
            "A9_ProvenanceChain".into(),
            "A10_ExternalAttestation".into(),
            "A11_TemporalValidity".into(),
            "A12_DependencyClosure".into(),
            "A13_ReplayProof".into(),
        ],
        gates_refused: Vec::new(),
        prior_receipt: None,
        signature: None,
        signing_key_fpr: None,
    }
}

struct Bag {
    scope_token: String,
    session_id: String,
    powl_string: String,
    powl_hash: [u8; 32],
    artifact_hash: String,
    required_stages: Vec<String>,
    observed_stages: Vec<String>,
}

fn ok_bag(scope_token: String, session: &str) -> Bag {
    let powl_string = "PO=(nodes={a, b}, order={a-->b})".to_string();
    let powl_hash = *blake3::hash(powl_string.as_bytes()).as_bytes();
    Bag {
        scope_token,
        session_id: session.to_string(),
        powl_string,
        powl_hash,
        artifact_hash: HEX32.to_string(),
        required_stages: vec!["a".into(), "b".into()],
        observed_stages: vec!["a".into(), "b".into()],
    }
}

fn run_cell_ready(
    bag: &Bag,
    store: &OcelStore,
    signature: Option<[u8; 64]>,
    fpr: Option<[u8; 8]>,
    trust: Option<&TrustedKeys>,
    allow_legacy: bool,
) -> Result<open_ontologies::receipts::Receipt, DefectClass> {
    let powl_ref = PowlOpRef {
        powl_string: &bag.powl_string,
        powl_hash: bag.powl_hash,
    };
    let provenance = vec![bag.artifact_hash.clone()];
    let granted = vec!["2026-05-08T00:00:00Z".to_string()];
    let admitted: Vec<String> = Vec::new();
    let inputs = CellReadyInputs {
        scope_token: &bag.scope_token,
        declared_powl: &powl_ref,
        ocel_trace_hash: HEX32,
        artifact_hash: &bag.artifact_hash,
        gate_config_hash: HEX32,
        session_revoked: false,
        fitness_observed: 0.99,
        precision_observed: 0.99,
        fitness_required: 0.95,
        precision_required: 0.85,
        required_stages: &bag.required_stages,
        observed_stages: &bag.observed_stages,
        conformance_run_id: "run-test",
        production_law_version: "ontostar-1.0.0",
        prior_receipt: None,
        session_id: &bag.session_id,
        provenance_evidence: &provenance,
        external_attestation: "",
        granted_at_chain: &granted,
        admitted_receipts: &admitted,
        replay_canonical_hash: HEX32,
        signature,
        signing_key_fpr: fpr,
        trusted_keys: trust,
        allow_legacy_unsigned: allow_legacy,
        trusted_keys_db: None,
        post_bootstrap: false,
        prior_tenant_receipt_count: 0,
    };
    cell_ready(inputs, store)
}

#[test]
fn signed_receipt_admits_under_real_verify_strict() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "ed25519-happy";
    let token = setup_scope(&db, session);
    let bag = ok_bag(token.clone(), session);

    let (signer, trust) = make_signer_and_trust();
    let preview = preview_record(&bag.scope_token, &bag.artifact_hash, &bag.powl_string);
    let msg = preview.canonical_bytes_for_signing();
    let sig = signer.sign(&msg).to_bytes();
    let fpr = signer.fingerprint();

    let receipt = run_cell_ready(&bag, &store, Some(sig), Some(fpr), Some(&trust), false)
        .expect("signed receipt must admit");
    assert_eq!(receipt.record.signature, Some(sig));
    assert_eq!(receipt.record.signing_key_fpr, Some(fpr));
}

#[test]
fn unsigned_receipt_passes_only_when_legacy_allowed() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "ed25519-legacy-allowed";
    let token = setup_scope(&db, session);
    let bag = ok_bag(token, session);

    let receipt = run_cell_ready(&bag, &store, None, None, None, true)
        .expect("legacy unsigned must pass when allow_legacy_unsigned=true");
    assert_eq!(receipt.record.signature, None);
    assert_eq!(receipt.record.signing_key_fpr, None);

    // OCEL must carry a `legacy_unsigned_receipt` audit event.
    let observed = store.observed_event_types_for_session(session).unwrap();
    assert!(
        observed.iter().any(|e| e == "legacy_unsigned_receipt"),
        "expected legacy_unsigned_receipt audit event in OCEL, got: {observed:?}"
    );
}

#[test]
fn unsigned_receipt_denied_when_legacy_disabled() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "ed25519-legacy-denied";
    let token = setup_scope(&db, session);
    let bag = ok_bag(token, session);

    match run_cell_ready(&bag, &store, None, None, None, false) {
        Err(DefectClass::AttestationMissing) => {}
        other => panic!("expected AttestationMissing, got {other:?}"),
    }
}

#[test]
fn unknown_signing_key_yields_attestation_invalid() {
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "ed25519-unknown-key";
    let token = setup_scope(&db, session);
    let bag = ok_bag(token, session);

    let (signer, _trust_a) = make_signer_and_trust();
    // Use a DIFFERENT trust set that does NOT contain the signer's
    // verifying key.
    let trust_b = TrustedKeys::new();
    let preview = preview_record(&bag.scope_token, &bag.artifact_hash, &bag.powl_string);
    let msg = preview.canonical_bytes_for_signing();
    let sig = signer.sign(&msg).to_bytes();
    let fpr = signer.fingerprint();

    match run_cell_ready(&bag, &store, Some(sig), Some(fpr), Some(&trust_b), false) {
        Err(DefectClass::AttestationInvalid { reason }) => {
            assert!(
                reason.starts_with("unknown_signing_key"),
                "expected unknown_signing_key reason, got {reason}"
            );
        }
        other => panic!("expected AttestationInvalid, got {other:?}"),
    }
}

#[test]
fn receipt_replay_attack_rejected() {
    // Round-2 cascade attack: a valid signature from receipt A is
    // pasted onto receipt B with a DIFFERENT artifact_hash. The
    // canonical_bytes_for_signing message changes (artifact_hash is
    // part of the signed bytes), so verify_strict must reject.
    let db = fresh_db();
    let store = OcelStore::new(db.clone());
    let session = "ed25519-replay-attack";
    let token = setup_scope(&db, session);

    let (signer, trust) = make_signer_and_trust();

    // Receipt A: artifact_hash = HEX32. Sign over its canonical bytes.
    let bag_a = ok_bag(token.clone(), session);
    let preview_a = preview_record(&bag_a.scope_token, &bag_a.artifact_hash, &bag_a.powl_string);
    let msg_a = preview_a.canonical_bytes_for_signing();
    let sig_a = signer.sign(&msg_a).to_bytes();
    let fpr = signer.fingerprint();

    // Sanity: receipt A admits cleanly.
    run_cell_ready(&bag_a, &store, Some(sig_a), Some(fpr), Some(&trust), false)
        .expect("receipt A must admit");

    // Receipt B: SAME scope, SAME signer, but DIFFERENT artifact_hash.
    // Replay attacker pastes sig_a onto B. Must fail.
    let mut bag_b = ok_bag(token.clone(), session);
    bag_b.artifact_hash = HEX32B.to_string();

    match run_cell_ready(&bag_b, &store, Some(sig_a), Some(fpr), Some(&trust), false) {
        Err(DefectClass::AttestationInvalid { reason }) => {
            assert_eq!(reason, "signature_invalid", "got reason: {reason}");
        }
        other => panic!(
            "receipt-replay attack must be rejected with AttestationInvalid, got {other:?}"
        ),
    }
}
