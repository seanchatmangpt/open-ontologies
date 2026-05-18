/**
 * spineTraceAdapter.ts
 *
 * Adapts ZOE LA Mobile OCEL route stage completion events to wasm4pm
 * SpineTraceRecord format for process mining (conformance checking, discovery).
 *
 * Source reference (READ ONLY — do not import from ~/wasm4pm directly):
 *   ~/wasm4pm/packages/observability/src/spine-bridge.ts
 *
 * Structural compatibility: SpineTraceRecord fields verified 2026-05-18.
 * Keep in sync manually when wasm4pm updates its record type.
 *
 * Connect Group route stages: packages/routes/connectGroupStages.ts
 * OCEL event types: packages/evidence/OcelEvents.ts
 *
 * LIVE-07 compatibility: includes `zoela.route.*` attributes alongside the
 * required `run.id` and `service.name` fields. `mcpp.atomvm.state` is omitted
 * (BEAM-specific; not applicable to ZOE LA Mobile TypeScript runtime).
 *
 * @example
 * const record = emitRouteStageTrace({
 *   eventType: "connect_group.interest_expressed",
 *   routeStageCode: "cg-interest-expressed",
 *   objectId: "550e8400-e29b-41d4-a716-446655440000",
 *   objectType: "GroupInterest",
 *   actionClass: "A1",
 *   runId: "run-001",
 * });
 * // record.kind === "event"
 * // record.name === "connect_group.interest_expressed"
 * // record.fields["zoela.route.action_class"] === "A1"
 * // record.fields["run.id"] === "run-001"
 * // record.fields["service.name"] === "wasm4pm.spine"
 *
 * @example
 * const refusal = emitConsentRefusalTrace({
 *   eventType: "connect_group.invite_accepted",
 *   routeStageCode: "cg-invite-accepted",
 *   objectId: "550e8400-e29b-41d4-a716-446655440001",
 *   objectType: "GroupInvite",
 *   actionClass: "A4",
 *   runId: "run-002",
 * });
 * // refusal.name === "connect_group.invite_accepted.refused"
 * // refusal.fields["zoela.route.refusal_gate"] === "ConsentGate"
 */

// ─────────────────────────────────────────────────────────────────────────────
// SpineTraceRecord — structural copy from
//   ~/wasm4pm/packages/observability/src/spine-bridge.ts
// Fields: kind, name, fields (with required 'run.id' and 'service.name'), ts_ns
// ─────────────────────────────────────────────────────────────────────────────

export interface SpineTraceRecord {
  /** Span kind — always `"event"` for ZOE LA route stage records. */
  kind: 'span_open' | 'span_record' | 'event';
  /** OCEL event type name (e.g. `"connect_group.interest_expressed"`). */
  name: string;
  /**
   * Attribute bag evaluated by LIVE correlation rules.
   * Open index signature allows ZOE LA–specific fields alongside the required
   * base fields (`run.id`, `service.name`).
   */
  fields: {
    /** Stable run identifier — required by all LIVE rules. */
    'run.id': string;
    /** Source component — always `"wasm4pm.spine"` for ZOE LA route spans. */
    'service.name': 'wasm4pm.streaming' | 'wasm4pm.spine';
    /** Additional ZOE LA route–specific and mcpp-specific attributes. */
    [key: string]: string | number | boolean;
  };
  /** Wall-clock nanoseconds since UNIX epoch (mirrors `TraceRecord.ts_ns`). */
  ts_ns: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// ZoelaRouteEvent — input type representing a ZOE LA route stage completion
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A ZOE LA Mobile route stage completion event.
 *
 * Emitted by each Connect Group route stage handler on successful completion.
 * The `eventType` should match the OCEL event type defined in OcelEvents.ts
 * (e.g. `"connect_group.invite_sent"`).
 *
 * `routeStageCode` should match the `code` field from connectGroupStages.ts
 * (e.g. `"cg-invite-sent"`).
 *
 * `actionClass` is the autonomic action class from the stage definition:
 *   - `"A0"` — observation only
 *   - `"A1"` — autonomic action (self-service)
 *   - `"A2"` — guided action (system-assisted)
 *   - `"A3"` — supervised action (human-in-loop)
 *   - `"A4"` — refusal / consent gate failure
 */
export interface ZoelaRouteEvent {
  /** OCEL event type, e.g. `"connect_group.interest_expressed"`. */
  eventType: string;
  /** Connect Group stage code, e.g. `"cg-interest-expressed"`. */
  routeStageCode: string;
  /** UUID of the primary object involved in this stage. */
  objectId: string;
  /** Object type, e.g. `"GroupInterest"`, `"GroupInvite"`. */
  objectType: string;
  /** Autonomic action class: `"A0"` | `"A1"` | `"A2"` | `"A3"` | `"A4"`. */
  actionClass: 'A0' | 'A1' | 'A2' | 'A3' | 'A4';
  /** UUID for this route execution (correlates all stages in one run). */
  runId: string;
  /** Override timestamp in nanoseconds since UNIX epoch. Defaults to `Date.now() * 1_000_000`. */
  tsNs?: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// emitRouteStageTrace — standard stage completion event
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Converts a ZOE LA route stage completion into a wasm4pm SpineTraceRecord.
 *
 * Use for all normal stage completions (A0–A3). For A4 consent refusals,
 * use `emitConsentRefusalTrace` instead.
 *
 * The emitted record is compatible with wasm4pm process mining tools:
 * conformance checking against the declared Connect Group POWL model,
 * process discovery from OCEL event logs, and LIVE correlation rules.
 *
 * Required `run.id` and `service.name` fields are always populated.
 * ZOE LA–specific attributes use the `zoela.route.*` namespace.
 */
export function emitRouteStageTrace(event: ZoelaRouteEvent): SpineTraceRecord {
  return {
    kind: 'event',
    name: event.eventType,
    ts_ns: event.tsNs ?? Date.now() * 1_000_000,
    fields: {
      'run.id': event.runId,
      'service.name': 'wasm4pm.spine',
      'zoela.route.stage_code': event.routeStageCode,
      'zoela.route.event_type': event.eventType,
      'zoela.route.action_class': event.actionClass,
      'zoela.route.object_id': event.objectId,
      'zoela.route.object_type': event.objectType,
    },
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// emitConsentRefusalTrace — A4 consent gate refusal event
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Converts a ZOE LA consent gate refusal into a wasm4pm SpineTraceRecord.
 *
 * Use when a member declines or fails a gate check (A4 action class), most
 * commonly at the ConsentGate on the `cg-invite-accepted` stage. The emitted
 * span name is `{eventType}.refused` so process mining tools can distinguish
 * refusals from normal completions in the OCEL log.
 *
 * The `zoela.route.action_class` is always `"A4"` regardless of the input
 * value — the caller signals refusal by choosing this function.
 *
 * The `zoela.route.refusal_gate` attribute is set to `"ConsentGate"` to
 * match the gate declared in connectGroupStages.ts for the invite-accepted stage.
 */
export function emitConsentRefusalTrace(event: ZoelaRouteEvent): SpineTraceRecord {
  return {
    kind: 'event',
    name: `${event.eventType}.refused`,
    ts_ns: event.tsNs ?? Date.now() * 1_000_000,
    fields: {
      'run.id': event.runId,
      'service.name': 'wasm4pm.spine',
      'zoela.route.stage_code': event.routeStageCode,
      'zoela.route.event_type': event.eventType,
      'zoela.route.action_class': 'A4',
      'zoela.route.object_id': event.objectId,
      'zoela.route.object_type': event.objectType,
      'zoela.route.refusal_gate': 'ConsentGate',
    },
  };
}
