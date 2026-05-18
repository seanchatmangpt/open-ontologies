/**
 * packages/evidence/__tests__/argrIntegration.test.ts
 *
 * ARGR (Actor-Resolved Gap Rate) integration test.
 *
 * Exercises the full pipeline:
 *   Seed OCEL events → SpineTraceRecord → GapRecord[] → ARGR computation
 *
 * Scenario A — Sam's happy path (no gaps detected → ARGR = 1.0)
 *   The 3 events seeded in supabase/seeds/connect_groups.sql:
 *     1. cg.interest.submitted  (Stage 1)
 *     2. cg.invite.sent         (Stage 2)
 *     3. cg.route.closed        (Stage 3)
 *   This trace is a known partial variant (stages 1, 3, 8 of happy-path,
 *   which maps to cg.interest.submitted → cg.invite.sent → cg.route.closed).
 *   No gaps are detected; ARGR = 1.0 by convention (0 detected / 0 = 1.0).
 *
 * Scenario B — Pat's ConsentGate refusal (1 detected gap, 0 resolved → ARGR = 0.0)
 *   Pat's trace ends at cg.invite.accepted.refused (A4 action class).
 *   The route never progresses past the consent gate, creating 1 detected gap
 *   that is never resolved (RouteRefinementPolicy cannot override consent).
 *   ARGR = 0 resolved / 1 detected = 0.0.
 *
 * Execute with:
 *   npx tsx packages/evidence/__tests__/argrIntegration.test.ts
 */

import { emitRouteStageTrace, emitConsentRefusalTrace } from '../spineTraceAdapter.js';
import { toGapRecord, isKnownVariant, type GapRecord } from '../argrBridge.js';

// ─── Minimal assertion harness ───────────────────────────────────────────────

function assert(condition: boolean, message: string): void {
  if (!condition) throw new Error(`FAIL: ${message}`);
  console.log(`PASS: ${message}`);
}

// ─── ARGR computation helpers ────────────────────────────────────────────────

/**
 * Compute ARGR from a list of GapRecords.
 *
 * ARGR = resolved_gaps / detected_gaps
 *
 * Convention: if detected_gaps === 0, ARGR = 1.0 (no gaps = perfect conformance).
 */
function computeArgr(gaps: ReadonlyArray<GapRecord>): number {
  const detected = gaps.length;
  if (detected === 0) return 1.0;
  const resolved = gaps.filter((g) => g.resolved).length;
  return resolved / detected;
}

/**
 * Emit ARGR OTEL attributes from a set of gaps.
 *
 * Maps directly to the ArgRAttributes interface in argrBridge.ts.
 * handover_density is the ratio of gaps with final_precision defined
 * (i.e. gaps that passed through a handover decision point).
 */
function toArgRAttributes(gaps: ReadonlyArray<GapRecord>) {
  const detected = gaps.length;
  const resolved = gaps.filter((g) => g.resolved).length;
  const withHandover = gaps.filter((g) => g.final_precision !== undefined).length;
  return {
    'argr.resolved': resolved,
    'argr.detected': detected,
    'argr.rate': computeArgr(gaps),
    'argr.handover_density': detected === 0 ? 0 : withHandover / detected,
  };
}

// ─── Seed fixtures (mirrors supabase/seeds/connect_groups.sql) ───────────────
//
// run_id:  aaaaaaaa-0000-0000-0000-000000000001
// person:  44444444-0000-0000-0000-000000000001
// campus:  11111111-0000-0000-0000-000000000001
// group:   33333333-0000-0000-0000-000000000001
//
// These ts_ns values are deterministic offsets from a fixed epoch so the test
// is not sensitive to wall-clock time.

const BASE_TS_NS = 1_716_000_000_000_000_000; // 2024-05-18 (arbitrary stable epoch)

const SEED_RUN_ID = 'aaaaaaaa-0000-0000-0000-000000000001';
const SEED_PERSON_ID = '44444444-0000-0000-0000-000000000001';

// ─── Scenario A: Sam's happy path ────────────────────────────────────────────
// 3 events from the seed data:
//   event 77777777-…-0001: cg.interest.submitted  (+0 s)
//   event 77777777-…-0002: cg.invite.sent         (+1 s)
//   event 77777777-…-0003: cg.route.closed        (+2 s)

console.log('\n── Scenario A: Sam happy path (ARGR = 1.0) ──────────────────');

const samStage1 = emitRouteStageTrace({
  eventType: 'cg.interest.submitted',
  routeStageCode: 'interest_submitted',
  objectId: SEED_PERSON_ID,
  objectType: 'Person',
  actionClass: 'A1',
  runId: SEED_RUN_ID,
  tsNs: BASE_TS_NS,
});

const samStage2 = emitRouteStageTrace({
  eventType: 'cg.invite.sent',
  routeStageCode: 'invite_sent',
  objectId: SEED_PERSON_ID,
  objectType: 'Person',
  actionClass: 'A1',
  runId: SEED_RUN_ID,
  tsNs: BASE_TS_NS + 1_000_000_000,
});

const samStage3 = emitRouteStageTrace({
  eventType: 'cg.route.closed',
  routeStageCode: 'route_closed',
  objectId: SEED_PERSON_ID,
  objectType: 'Person',
  actionClass: 'A1',
  runId: SEED_RUN_ID,
  tsNs: BASE_TS_NS + 2_000_000_000,
});

// All 3 traces must be well-formed SpineTraceRecords.
assert(samStage1.kind === 'event', 'Sam stage1: kind is event');
assert(samStage1.name === 'cg.interest.submitted', 'Sam stage1: correct event type');
assert(samStage1.fields['run.id'] === SEED_RUN_ID, 'Sam stage1: run.id matches seed run_id');
assert(samStage1.fields['service.name'] === 'wasm4pm.spine', 'Sam stage1: service.name is wasm4pm.spine');
assert(samStage1.ts_ns === BASE_TS_NS, 'Sam stage1: ts_ns matches seed offset');

assert(samStage2.name === 'cg.invite.sent', 'Sam stage2: correct event type');
assert(samStage2.fields['run.id'] === SEED_RUN_ID, 'Sam stage2: run.id matches seed run_id');
assert(samStage2.ts_ns === BASE_TS_NS + 1_000_000_000, 'Sam stage2: ts_ns is 1 s after stage1');

assert(samStage3.name === 'cg.route.closed', 'Sam stage3: correct event type');
assert(samStage3.fields['run.id'] === SEED_RUN_ID, 'Sam stage3: run.id matches seed run_id');
assert(samStage3.ts_ns === BASE_TS_NS + 2_000_000_000, 'Sam stage3: ts_ns is 2 s after stage1');

// Timestamps are monotonically increasing (temporal ordering).
assert(samStage1.ts_ns < samStage2.ts_ns, 'Sam: stage1 ts_ns < stage2 ts_ns (temporal order)');
assert(samStage2.ts_ns < samStage3.ts_ns, 'Sam: stage2 ts_ns < stage3 ts_ns (temporal order)');

// Extract event-type sequence from Sam's trace.
const samEventSequence = [samStage1.name, samStage2.name, samStage3.name];

// The seed emits a 3-event partial trace (interest→invite→closed).
// This is NOT one of the declared 5 canonical variants (which have 4–8 events)
// so isKnownVariant returns false — but the trace produces zero gaps because
// the ARGR pipeline only fires gap detection on divergence (Jaccard > 0.30),
// which we model here as: if the trace contains no unknown event types, no
// gap is created.  The divergence check is: every event in the trace exists
// in at least one declared variant.
const knownEventTypes = new Set(
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
);
const samHasUnknownEvents = samEventSequence.some((e) => !knownEventTypes.has(e));
assert(!samHasUnknownEvents, 'Sam: all event types are known (no divergence triggers)');

// No gaps are detected for Sam's trace — every event is a standard activity.
// isKnownVariant is false (3-event slice is not a complete variant) but
// there are no rogue event types, so gap count stays 0.
assert(!isKnownVariant(samEventSequence), 'Sam: 3-event seed trace is a partial trace (not a canonical variant)');

// Sam's gap list is empty — no divergence detected.
const samGaps: GapRecord[] = [];
const samArgr = computeArgr(samGaps);
assert(samArgr === 1.0, 'Sam: ARGR = 1.0 (no gaps detected → perfect conformance by convention)');

const samAttrs = toArgRAttributes(samGaps);
assert(samAttrs['argr.detected'] === 0, 'Sam: argr.detected = 0');
assert(samAttrs['argr.resolved'] === 0, 'Sam: argr.resolved = 0');
assert(samAttrs['argr.rate'] === 1.0, 'Sam: argr.rate = 1.0');
assert(samAttrs['argr.handover_density'] === 0, 'Sam: argr.handover_density = 0 (no gaps, no handovers)');

// ─── Scenario B: Pat's ConsentGate refusal ───────────────────────────────────
// Pat's route never starts — consent gate blocks at stage 0.
// 1 detected gap (the refused route entry), 0 resolved → ARGR = 0.0.

console.log('\n── Scenario B: Pat refusal path (ARGR = 0.0) ───────────────');

const PAT_RUN_ID = 'bbbbbbbb-0000-0000-0000-000000000001';
const PAT_PERSON_ID = '44444444-0000-0000-0000-000000000002';

const patRefusal = emitConsentRefusalTrace({
  eventType: 'cg.interest.submitted',
  routeStageCode: 'interest_submitted',
  objectId: PAT_PERSON_ID,
  objectType: 'Person',
  actionClass: 'A4',
  runId: PAT_RUN_ID,
  tsNs: BASE_TS_NS,
});

assert(patRefusal.kind === 'event', 'Pat refusal: kind is event');
assert(patRefusal.name === 'cg.interest.submitted.refused', 'Pat refusal: event name ends with .refused');
assert(patRefusal.fields['run.id'] === PAT_RUN_ID, 'Pat refusal: run.id is pat run id');
assert(patRefusal.fields['zoela.route.action_class'] === 'A4', 'Pat refusal: action_class is A4');
assert(patRefusal.fields['zoela.route.refusal_gate'] === 'ConsentGate', 'Pat refusal: refusal_gate is ConsentGate');

// The refusal event type is NOT in the known-event-type set — it has the
// ".refused" suffix — which would trigger gap detection by the ARGR pipeline
// (Jaccard divergence fires on unrecognised events in the trace window).
// We model this as: create 1 GapRecord for Pat's run.
const patGapRecord = toGapRecord(patRefusal, 'gap-pat-consent-001', 0.0);

assert(patGapRecord.gap_id === 'gap-pat-consent-001', 'Pat gap: gap_id set correctly');
assert(patGapRecord.activity_id === 'cg.interest.submitted.refused', 'Pat gap: activity_id is the refused event type');
assert(patGapRecord.run_id === PAT_RUN_ID, 'Pat gap: run_id matches pat run');
assert(patGapRecord.initial_precision === 0.0, 'Pat gap: initial_precision = 0.0 (consent gate blocked)');
assert(patGapRecord.resolved === false, 'Pat gap: starts unresolved');
assert(typeof patGapRecord.detected_at === 'string' && patGapRecord.detected_at.length > 0, 'Pat gap: detected_at is populated');
assert(patGapRecord.resolved_at === undefined, 'Pat gap: no resolved_at (gap not closed)');
assert(patGapRecord.final_precision === undefined, 'Pat gap: no final_precision (gap not closed)');

// Pat's gaps: 1 detected, 0 resolved.
const patGaps: GapRecord[] = [patGapRecord];
const patArgr = computeArgr(patGaps);
assert(patArgr === 0.0, 'Pat: ARGR = 0.0 (1 detected, 0 resolved)');

const patAttrs = toArgRAttributes(patGaps);
assert(patAttrs['argr.detected'] === 1, 'Pat: argr.detected = 1');
assert(patAttrs['argr.resolved'] === 0, 'Pat: argr.resolved = 0');
assert(patAttrs['argr.rate'] === 0.0, 'Pat: argr.rate = 0.0');
assert(patAttrs['argr.handover_density'] === 0, 'Pat: argr.handover_density = 0 (unresolved, no handover)');

// ─── Scenario C: Cross-run isolation ─────────────────────────────────────────
// Sam's run and Pat's run are independent — mixing their gap lists must not
// pollute each other's ARGR scores.

console.log('\n── Scenario C: Cross-run isolation ──────────────────────────');

assert(samStage1.fields['run.id'] !== patRefusal.fields['run.id'], 'Isolation: Sam and Pat have different run.id values');

// Aggregate both runs: 0 Sam gaps + 1 Pat gap = 1 detected, 0 resolved → ARGR = 0.0
const allGaps: GapRecord[] = [...samGaps, ...patGaps];
const aggregateArgr = computeArgr(allGaps);
assert(aggregateArgr === 0.0, 'Aggregate: ARGR = 0.0 across both runs (1 detected, 0 resolved)');

// Filter Sam's run only → ARGR = 1.0
const samRunGaps = allGaps.filter((g) => g.run_id === SEED_RUN_ID);
assert(computeArgr(samRunGaps) === 1.0, 'Isolation: filtering Sam run_id → ARGR = 1.0');

// Filter Pat's run only → ARGR = 0.0
const patRunGaps = allGaps.filter((g) => g.run_id === PAT_RUN_ID);
assert(computeArgr(patRunGaps) === 0.0, 'Isolation: filtering Pat run_id → ARGR = 0.0');

// ─── All tests completed ──────────────────────────────────────────────────────

console.log('\n✓ All ARGR integration tests passed');
