// ZOE LA Mobile — Autonomic Action Classes (A0-A4)
// Derived from: ontology/zoela/connect-group-routes.ttl
// Doctrine: Human action is not the default. Autonomic action is default when route law permits.

/** A0: Observe only — no mutation, no side effects */
export const A0_OBSERVE = "A0" as const;
/** A1: Safe autonomic action — execute immediately, no human required */
export const A1_SAFE = "A1" as const;
/** A2: Reversible autonomic action — execute with rollback capability */
export const A2_REVERSIBLE = "A2" as const;
/** A3: Human approval required — escalate before execution */
export const A3_HUMAN = "A3" as const;
/** A4: Refuse — missing consent, unsafe access, or opt-out violation */
export const A4_REFUSE = "A4" as const;

export type AutonomicActionClass =
  | typeof A0_OBSERVE
  | typeof A1_SAFE
  | typeof A2_REVERSIBLE
  | typeof A3_HUMAN
  | typeof A4_REFUSE;

export interface AutonomicAction {
  actionClass: AutonomicActionClass;
  routeStageCode: string;
  eventType: string;
  description: string;
  reversible: boolean;
  humanRequired: boolean;
}

/**
 * 8-conjunct AutonomicActionAllowed equation:
 * RouteEnabled ∧ PolicyPermits ∧ ConsentSatisfied ∧ EvidenceSufficient
 * ∧ RoleAuthoritySatisfied ∧ RiskWithinBand ∧ ReversibilityAllowed ∧ HumanPresenceNotRequired
 */
export interface AdmissionDecision {
  allowed: boolean;
  actionClass: AutonomicActionClass;
  refusalGate?: AdmissionGateCode;
  refusalReason?: string;
  evidence: Record<string, boolean>;
}

export type AdmissionGateCode =
  | "RouteEnabledGate"
  | "ConsentGate"
  | "RoleGate"
  | "CapacityGate"
  | "ScheduleGate"
  | "PolicyGate"
  | "NotificationBudgetGate";

export interface AdmissionGateResult {
  gate: AdmissionGateCode;
  passed: boolean;
  reason?: string;
}

/** Evaluate all 8 conjuncts — first failing gate determines refusal */
export function evaluateAdmission(
  gates: AdmissionGateResult[]
): AdmissionDecision {
  for (const gate of gates) {
    if (!gate.passed) {
      return {
        allowed: false,
        actionClass: gate.gate === "ConsentGate" ? A4_REFUSE : A4_REFUSE,
        refusalGate: gate.gate,
        refusalReason: gate.reason ?? `${gate.gate} failed`,
        evidence: Object.fromEntries(gates.map(g => [g.gate, g.passed])),
      };
    }
  }
  return {
    allowed: true,
    actionClass: A1_SAFE,
    evidence: Object.fromEntries(gates.map(g => [g.gate, g.passed])),
  };
}
