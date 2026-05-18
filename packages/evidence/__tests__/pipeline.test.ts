/**
 * packages/evidence/__tests__/pipeline.test.ts
 *
 * End-to-end integration: ZOE LA OCEL → ARGR → mcpp proof chain
 *
 * Execute with:
 *   npx ts-node --esm packages/evidence/__tests__/pipeline.test.ts
 * or compile-check only:
 *   npx tsc --noEmit
 */

import { emitRouteStageTrace, emitConsentRefusalTrace } from '../spineTraceAdapter.js';
import { toGapRecord, CONNECT_GROUP_OCEL_VARIANTS } from '../argrBridge.js';
import { toMcppVerdict, CONFORMANCE_REQUIREMENT } from '../mcppProofChain.js';
import { evaluateConnectGroupAdmission } from '../../autonomics/connectGroupAdmission.js';

// ─── Minimal assertion harness (no test framework required) ─────────────────

function assert(condition: boolean, message: string): void {
  if (!condition) throw new Error(`FAIL: ${message}`);
  console.log(`PASS: ${message}`);
}

// ─── Test 1: Happy path — Sam WithConsent ───────────────────────────────────

const samContext = {
  personId: '44444444-0000-0000-0000-000000000001',
  campusId: '11111111-0000-0000-0000-000000000001',
  hasConsent: true,
  hasRole: true,
  groupHasCapacity: true,
  scheduleMatches: true,
  isPrivateGroup: false,
  withinNotificationBudget: true,
  routeEnabled: true,
};

const samDecision = evaluateConnectGroupAdmission(samContext);
assert(samDecision.allowed, 'Sam: admission allowed');
assert(samDecision.actionClass === 'A1', 'Sam: action class is A1');

const samTrace = emitRouteStageTrace({
  eventType: 'cg.interest.submitted',
  routeStageCode: 'CGStageInterestSubmitted',
  objectId: samContext.personId,
  objectType: 'GroupInterest',
  actionClass: 'A1',
  runId: 'run-sam-001',
});
assert(samTrace.kind === 'event', 'Sam trace: kind is event');
assert(samTrace.name === 'cg.interest.submitted', 'Sam trace: correct event type');
assert(typeof samTrace.fields['zoela.route.stage_code'] === 'string', 'Sam trace: has stage_code');
assert(samTrace.fields['run.id'] === 'run-sam-001', 'Sam trace: correct run.id');
assert(samTrace.fields['service.name'] === 'wasm4pm.spine', 'Sam trace: service.name is wasm4pm.spine');
assert(samTrace.fields['zoela.route.action_class'] === 'A1', 'Sam trace: action class A1 in fields');

const samVerdict = toMcppVerdict(samDecision);
assert(samVerdict.type === 'Accept', 'Sam: verdict is Accept');
if (samVerdict.type === 'Accept') {
  assert(samVerdict.conformance.fitness === 1.0, 'Sam: Accept carries fitness == 1.0');
  assert(samVerdict.conformance.precision === 1.0, 'Sam: Accept carries precision == 1.0');
}

// ─── Test 2: Refusal path — Pat NoConsent ─────────────────────────────────

const patContext = { ...samContext, personId: '44444444-0000-0000-0000-000000000002', hasConsent: false };
const patDecision = evaluateConnectGroupAdmission(patContext);
assert(!patDecision.allowed, 'Pat: admission refused');
assert(patDecision.actionClass === 'A4', 'Pat: action class is A4');
assert(patDecision.refusalGate === 'ConsentGate', 'Pat: refused at ConsentGate');

const patTrace = emitConsentRefusalTrace({
  eventType: 'cg.interest.submitted',
  routeStageCode: 'CGStageInterestSubmitted',
  objectId: patContext.personId,
  objectType: 'GroupInterest',
  actionClass: 'A4',
  runId: 'run-pat-001',
});
assert(patTrace.name === 'cg.interest.submitted.refused', 'Pat trace: refusal event name');
assert(patTrace.fields['zoela.route.refusal_gate'] === 'ConsentGate', 'Pat trace: ConsentGate in refusal field');
assert(patTrace.fields['zoela.route.action_class'] === 'A4', 'Pat trace: action class is A4');

const patVerdict = toMcppVerdict(patDecision);
assert(patVerdict.type === 'Refuse', 'Pat: verdict is Refuse');
if (patVerdict.type === 'Refuse') {
  assert(patVerdict.reason.namespace === 'extension/zoela-mobile', 'Pat: AndonReason namespace is extension/zoela-mobile');
  assert(patVerdict.reason.code === 'ConsentGate', 'Pat: AndonReason code is ConsentGate (gate name)');
  assert(patVerdict.deviations.length === 0, 'Pat: deviations array is empty');
}

// ─── Test 3: GapRecord from SpineTraceRecord ──────────────────────────────

const gap = toGapRecord(samTrace, 'gap-ulid-001', 0.72);
assert(gap.gap_id === 'gap-ulid-001', 'GapRecord: gap_id matches');
assert(gap.activity_id === 'cg.interest.submitted', 'GapRecord: activity_id is the OCEL event type');
assert(gap.run_id === 'run-sam-001', 'GapRecord: run_id matches trace run.id');
assert(gap.initial_precision === 0.72, 'GapRecord: initial_precision preserved');
assert(gap.resolved === false, 'GapRecord: starts unresolved');
assert(typeof gap.detected_at === 'string' && gap.detected_at.length > 0, 'GapRecord: detected_at is a non-empty string');

// ─── Test 4: OCEL variants ────────────────────────────────────────────────

assert(CONNECT_GROUP_OCEL_VARIANTS.length >= 2, 'OCEL variants: at least 2 paths defined');
assert(CONNECT_GROUP_OCEL_VARIANTS[0].length === 8, 'OCEL variants: happy path has 8 events');
assert(CONNECT_GROUP_OCEL_VARIANTS[0][0] === 'cg.interest.submitted', 'OCEL variants: happy path starts with interest submitted');
assert(CONNECT_GROUP_OCEL_VARIANTS[0][7] === 'cg.route.closed', 'OCEL variants: happy path ends with route closed');
assert(CONNECT_GROUP_OCEL_VARIANTS[1][3] === 'cg.invite.accepted.refused', 'OCEL variants: refusal path ends at refused invite');

// ─── Test 5: Conformance requirement (K-P09 compliance) ───────────────────

assert(CONFORMANCE_REQUIREMENT.fitness === 1.0, 'mcpp: fitness must be 1.0');
assert(CONFORMANCE_REQUIREMENT.precision === 1.0, 'mcpp: precision must be 1.0');
assert(CONFORMANCE_REQUIREMENT.lifecycle === 1.0, 'mcpp: lifecycle must be 1.0');
assert(CONFORMANCE_REQUIREMENT.cardinality === 1.0, 'mcpp: cardinality must be 1.0');
assert(CONFORMANCE_REQUIREMENT.receipt === 1.0, 'mcpp: receipt must be 1.0');

// ─── All tests completed ──────────────────────────────────────────────────

console.log('\n✓ All pipeline integration tests passed');
