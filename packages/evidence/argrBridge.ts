/**
 * argrBridge.ts
 *
 * Bridges ZOE LA Mobile OCEL events to the wasm4pm ARGR (Actor-Resolved Gap
 * Rate) metric format for process mining gap detection and resolution tracking.
 *
 * Source reference (READ ONLY — do not import from ~/wasm4pm directly):
 *   ~/wasm4pm/packages/observability/src/argr.ts
 *
 * ARGR is the ratio of resolved POWL activity gaps to detected gaps.
 * In ZOE LA terms a "gap" is a Connect Group route stage that:
 *   - Was detected as a divergence (Jaccard distance > 0.30 from the declared
 *     POWL model), AND
 *   - May subsequently be resolved when the RouteRefinementPolicy restores
 *     conformance precision above the threshold.
 *
 * Structural compatibility: GapRecord fields verified 2026-05-18.
 * Keep in sync manually when wasm4pm updates its types.
 *
 * @example
 * // Detect a gap on the cg-invite-sent stage:
 * const gap = toGapRecord(spineRecord, "gap-ulid-001", 0.45);
 * tracker.recordDetected(gap.gap_id, gap.activity_id, gap.run_id, gap.initial_precision);
 *
 * @example
 * // Resolve the gap after RouteRefinementPolicy restores conformance:
 * tracker.recordResolved("gap-ulid-001", 0.88);
 */

import type { SpineTraceRecord } from './spineTraceAdapter.js';

// ─────────────────────────────────────────────────────────────────────────────
// GapRecord — structural copy from
//   ~/wasm4pm/packages/observability/src/argr.ts
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A single gap detected by the drift pre-screen and optionally resolved by
 * the RouteRefinementPolicy.
 *
 * Field names are a structural copy of wasm4pm's GapRecord interface.
 * Any change to wasm4pm's type must be reflected here manually.
 */
export interface GapRecord {
  /** Unique identifier for this gap (e.g. ULID or UUID). */
  gap_id: string;
  /** The POWL activity IRI/ID where the gap was detected. */
  activity_id: string;
  /** ISO-8601 timestamp when the gap was first detected. */
  detected_at: string;
  /** run_id of the trace that triggered detection. */
  run_id: string;
  /** Conformance precision score at the moment of detection. */
  initial_precision: number;
  /** Whether the RouteRefinementPolicy has closed this gap. */
  resolved: boolean;
  /** ISO-8601 timestamp when the gap was resolved (if resolved). */
  resolved_at?: string;
  /** Conformance precision score after successful resolution. */
  final_precision?: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// OtelAttributes — ARGR OTEL span attribute shape
// ─────────────────────────────────────────────────────────────────────────────

/**
 * ARGR OTEL-compatible flat attributes emitted by ArgRTracker.toOtelAttributes().
 *
 * All four keys are required; values are computed from the gap accumulator.
 */
export interface ArgRAttributes {
  'argr.resolved': number;
  'argr.detected': number;
  'argr.rate': number;
  'argr.handover_density': number;
}

// ─────────────────────────────────────────────────────────────────────────────
// toGapRecord — SpineTraceRecord → GapRecord
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Converts a ZOE LA Mobile SpineTraceRecord into a wasm4pm GapRecord.
 *
 * The `activity_id` is derived from the spine record's `name` field, which
 * carries the OCEL event type (e.g. `"connect_group.invite_sent"`).
 *
 * The caller supplies `gapId` (a ULID or UUID) and `initialPrecision` (the
 * conformance precision score at the moment the drift pre-screen fired).
 * Gaps start unresolved; call `ArgRTracker.recordResolved()` separately when
 * the RouteRefinementPolicy closes the gap.
 *
 * @param record           The SpineTraceRecord emitted by the ZOE LA route stage.
 * @param gapId            Unique gap identifier (ULID recommended).
 * @param initialPrecision Conformance precision score at detection (0.0–1.0).
 * @returns                A GapRecord ready for `ArgRTracker.recordDetected()`.
 */
export function toGapRecord(
  record: SpineTraceRecord,
  gapId: string,
  initialPrecision: number,
): GapRecord {
  return {
    gap_id: gapId,
    activity_id: record.name,
    detected_at: new Date(record.ts_ns / 1_000_000).toISOString(),
    run_id: record.fields['run.id'],
    initial_precision: initialPrecision,
    resolved: false,
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// CONNECT_GROUP_OCEL_VARIANTS
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Expected OCEL event-type sequences (variants) for the Connect Group Join
 * Route.  Each inner array is one variant — a complete or partial trace through
 * the 8-stage route as defined in connectGroupStages.ts.
 *
 * These variants are the reference model for ARGR gap detection: a trace that
 * deviates from every listed variant triggers a gap observation.
 *
 * Variant taxonomy:
 *   [0] Happy path — all 8 stages complete in order
 *   [1] Consent refusal at invite-accepted gate (A4, ConsentGate)
 *   [2] Waitlist path — group full, spot reserved on waitlist
 *   [3] No-show path — spot reserved but attendance not recorded
 *   [4] Follow-up only — attendance recorded, follow-up created, no route close
 */
export const CONNECT_GROUP_OCEL_VARIANTS: ReadonlyArray<ReadonlyArray<string>> = [
  // [0] Happy path: all 8 stages in sequence
  [
    'cg.interest.submitted',
    'cg.groups.matched',
    'cg.invite.sent',
    'cg.invite.accepted',
    'cg.spot.reserved',
    'cg.attendance.recorded',
    'cg.followup.created',
    'cg.route.closed',
  ],
  // [1] Consent refusal at invite-accepted gate (A4)
  [
    'cg.interest.submitted',
    'cg.groups.matched',
    'cg.invite.sent',
    'cg.invite.accepted.refused',
  ],
  // [2] Waitlist path — group full, no immediate spot
  [
    'cg.interest.submitted',
    'cg.groups.matched',
    'cg.invite.sent',
    'cg.invite.accepted',
    'cg.spot.reserved.waitlisted',
  ],
  // [3] No-show path — reserved spot, no attendance
  [
    'cg.interest.submitted',
    'cg.groups.matched',
    'cg.invite.sent',
    'cg.invite.accepted',
    'cg.spot.reserved',
    'cg.route.closed',
  ],
  // [4] Follow-up only — attended but route not formally closed
  [
    'cg.interest.submitted',
    'cg.groups.matched',
    'cg.invite.sent',
    'cg.invite.accepted',
    'cg.spot.reserved',
    'cg.attendance.recorded',
    'cg.followup.created',
  ],
] as const;

// ─────────────────────────────────────────────────────────────────────────────
// isKnownVariant — variant membership test
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Returns `true` if the supplied event-type sequence exactly matches one of
 * the known Connect Group variants in {@link CONNECT_GROUP_OCEL_VARIANTS}.
 *
 * Use this as a fast pre-check before running a full ARGR gap detection pass:
 * if the trace is a known variant, no gap observation is needed.
 *
 * @param trace Array of OCEL event type strings in emission order.
 */
export function isKnownVariant(trace: ReadonlyArray<string>): boolean {
  return CONNECT_GROUP_OCEL_VARIANTS.some(
    (variant) =>
      variant.length === trace.length &&
      variant.every((eventType, i) => eventType === trace[i]),
  );
}
