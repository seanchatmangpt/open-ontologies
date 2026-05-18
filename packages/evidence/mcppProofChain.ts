/**
 * packages/evidence/mcppProofChain.ts
 *
 * TypeScript bridge between ZOE LA Mobile receipts and mcpp's proof chain format.
 *
 * Structural copies of mcpp types are derived from:
 *   ~/mcpp/crates/mcpp-core/src/protocol/verdict.rs  — Verdict enum
 *   ~/mcpp/crates/mcpp-core/src/receipt.rs            — BuildReceipt struct
 *   ~/mcpp/crates/mcpp-core/src/protocol/andon.rs     — AndonReason struct
 *
 * K-P09 doctrine: Only Verdict::Accept carries a receipt. Verdict::Refuse
 * emits zero receipts. Conformance == 1.0 exactly on every dimension.
 */

import type {
  ResourceDistributedReceiptReceipt,
  CareRouteClosedReceiptReceipt,
} from './receipts.js';
import type { AdmissionDecision } from '../autonomics/autonomicActions.js';

// ── ZoelaReceipt union — all receipt shapes emitted by ZOE LA ──────────────

export type ZoelaReceipt =
  | ResourceDistributedReceiptReceipt
  | CareRouteClosedReceiptReceipt;

// ── Structural copy: mcpp AndonReason ─────────────────────────────────────
//
// Source: mcpp-core/src/protocol/andon.rs
//   pub struct AndonReason {
//     pub namespace:    String,
//     pub code:         String,
//     pub detail:       Option<String>,
//     pub evidence_ref: Option<String>,
//   }

export interface McppAndonReason {
  namespace: string;
  code: string;
  detail?: string;
  evidence_ref?: string;
}

// ── Structural copy: mcpp Deviation ──────────────────────────────────────
//
// Source: mcpp-core/src/protocol/response.rs (referenced by Verdict::Refuse)

export interface McppDeviation {
  dimension: string;
  observed?: number;
  required: number;
}

// ── Structural copy: mcpp Verdict ─────────────────────────────────────────
//
// Source: mcpp-core/src/protocol/verdict.rs
//   pub enum Verdict {
//     Accept(AcceptCertificate),
//     Refuse {
//       reason:     AndonReason,
//       deviations: Vec<Deviation>,
//     },
//   }
//
// AcceptCertificate carries ConformanceThresholds (all dimensions verified == 1.0
// before issue). The _seal: () field is a Rust compile-time K-P09 guard with no
// TypeScript equivalent; its enforcement is documented here for traceability.

export interface McppConformanceThresholds {
  fitness?: number;
  precision?: number;
  lifecycle?: number;
  cardinality?: number;
  receipt?: number;
}

export type McppVerdict =
  | {
      type: 'Accept';
      /** Mirrors AcceptCertificate — conformance vector verified == 1.0 on every
       *  dimension by ProofWriter::admit before this variant was constructed.
       *  In Rust the _seal: () field prevents any code path outside ProofWriter
       *  from constructing Accept. TypeScript callers must honour the same
       *  discipline: only toMcppVerdict() with allowed == true may produce this. */
      conformance: McppConformanceThresholds;
    }
  | {
      type: 'Refuse';
      reason: McppAndonReason;
      deviations: McppDeviation[];
    };

// ── Structural copy: mcpp BuildReceipt ────────────────────────────────────
//
// Source: mcpp-core/src/receipt.rs
//   pub struct BuildReceipt {
//     pub receipt_id:          String,
//     pub timestamp:           String,
//     pub part_name:           String,
//     pub manifest_hash:       String,
//     pub wasm_hash:           Option<String>,
//     pub fixture_hashes:      Vec<String>,
//     pub route_id:            Option<String>,
//     pub prev_receipt_hash:   Option<String>,
//     pub signing_key_fpr:     Option<String>,
//     pub signature:           Option<String>,
//     pub receipt_schema:      String,
//     // v1.2 (Slice C) optional fields:
//     pub route_stage:         Option<String>,
//     pub missing_obligations: Vec<String>,
//     pub policy_refs:         Vec<String>,
//     pub evidence_refs:       Vec<String>,
//     pub refusal_reason:      Option<AndonReason>,
//     pub argr_at_emission:    Option<f64>,
//   }

export interface McppBuildReceipt {
  receipt_id: string;
  timestamp: string;
  part_name: string;
  manifest_hash: string;
  wasm_hash?: string;
  fixture_hashes: string[];
  route_id?: string;
  prev_receipt_hash?: string;
  signing_key_fpr?: string;
  signature?: string;
  receipt_schema: string;
  // v1.2 (Slice C) optional fields
  route_stage?: string;
  missing_obligations: string[];
  policy_refs: string[];
  evidence_refs: string[];
  refusal_reason?: McppAndonReason;
  argr_at_emission?: number;
}

// ── mcpp receipt schema version this bridge targets ───────────────────────
//
// Source: mcpp-core/src/receipt.rs  pub const RECEIPT_SCHEMA: &str = "1.2";

export const MCPP_RECEIPT_SCHEMA = '1.2' as const;

// ── K-P09 conformance requirement ─────────────────────────────────────────
//
// Source: proof_writer.rs — every dimension must be Some(1.0) bit-exact.
// "0.999 is still an Andon pull. 1.0 means the gauge fits."

export const CONFORMANCE_REQUIREMENT = {
  fitness: 1.0,
  precision: 1.0,
  lifecycle: 1.0,
  cardinality: 1.0,
  receipt: 1.0,
} as const satisfies McppConformanceThresholds;

// ── toMcppReceipt ──────────────────────────────────────────────────────────
//
// Maps a ZOE LA receipt to an mcpp BuildReceipt.
//
// Field mapping:
//   zoelaReceipt.receiptId          → receipt_id
//   zoelaReceipt.issuedAt           → timestamp
//   zoelaReceipt.routeInstanceId    → route_id
//   zoelaReceipt.blake3Hash         → manifest_hash (BLAKE3 content digest)
//   zoelaReceipt.routeStageCode     → route_stage   (v1.2 field)
//   zoelaReceipt.evidenceIds        → evidence_refs  (v1.2 field)
//   zoelaReceipt.ocelEventType      → fixture_hashes[0] (OCEL event type as
//                                     fixture reference; a real hash is
//                                     substituted by the signing step)
//   zoelaReceipt.subjectId          → part_name (the object under care)

export function toMcppReceipt(zoelaReceipt: ZoelaReceipt): McppBuildReceipt {
  return {
    receipt_id: zoelaReceipt.receiptId,
    timestamp: zoelaReceipt.issuedAt,
    part_name: zoelaReceipt.subjectId,
    manifest_hash: zoelaReceipt.blake3Hash,
    wasm_hash: undefined,
    fixture_hashes: [zoelaReceipt.ocelEventType],
    route_id: zoelaReceipt.routeInstanceId,
    prev_receipt_hash: undefined,
    signing_key_fpr: undefined,
    signature: undefined,
    receipt_schema: MCPP_RECEIPT_SCHEMA,
    // v1.2 fields
    route_stage: zoelaReceipt.routeStageCode,
    missing_obligations: [],
    policy_refs: [],
    evidence_refs: zoelaReceipt.evidenceIds,
    refusal_reason: undefined,
    argr_at_emission: undefined,
  };
}

// ── toMcppVerdict ──────────────────────────────────────────────────────────
//
// Maps a ZOE LA AdmissionDecision to an mcpp Verdict.
//
// ZOE LA gate → mcpp Verdict mapping:
//   A1_SAFE / A2_REVERSIBLE (allowed == true)
//     → Verdict::Accept with CONFORMANCE_REQUIREMENT
//   A4_REFUSE (ConsentGate / CapacityGate, allowed == false)
//     → Verdict::Refuse with AndonReason in extension/zoela-mobile namespace
//   A3_HUMAN (PolicyGate, allowed == false)
//     → Verdict::Refuse with AndonReason code "HumanApprovalRequired"
//   A0_OBSERVE
//     → Verdict::Refuse with AndonReason code "ObserveOnly" (no state change)
//
// Note: Verdict::Accept emits exactly 1 receipt. Verdict::Refuse emits 0.
// This is the K-P09 receipt cardinality constraint.

export function toMcppVerdict(decision: AdmissionDecision): McppVerdict {
  if (decision.allowed) {
    return {
      type: 'Accept',
      conformance: { ...CONFORMANCE_REQUIREMENT },
    };
  }

  // Map ZOE LA gate refusal to an AndonReason in the extension/zoela-mobile namespace.
  // The mcpp: namespace is CLOSED at 19 codes — ZOE LA reasons live in extension/.
  const gate = decision.refusalGate ?? 'UnknownGate';
  const detail = decision.refusalReason ?? `${gate} failed`;

  const andonCode = (() => {
    switch (decision.actionClass) {
      case 'A3': return 'HumanApprovalRequired';
      case 'A0': return 'ObserveOnly';
      default:   return gate; // A4: use gate name as code (ConsentGate, CapacityGate, etc.)
    }
  })();

  const reason: McppAndonReason = {
    namespace: 'extension/zoela-mobile',
    code: andonCode,
    detail,
  };

  return {
    type: 'Refuse',
    reason,
    deviations: [],
  };
}
