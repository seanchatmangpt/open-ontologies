// ZOE LA Mobile — Connect Group Join Route Admission Gates
// Derived from: ontology/zoela/connect-group-routes.ttl
// Route: ConnectGroupJoinRoute — first ZOE LA autonomic route

import {
  AdmissionGateCode,
  AdmissionGateResult,
  AdmissionDecision,
  evaluateAdmission,
  A3_HUMAN,
} from "./autonomicActions.js";

export interface ConnectGroupContext {
  personId: string;
  campusId: string;
  hasConsent: boolean;
  hasRole: boolean;
  groupHasCapacity: boolean;
  scheduleMatches: boolean;
  isPrivateGroup: boolean;
  withinNotificationBudget: boolean;
  routeEnabled: boolean;
}

/** Ordered gate evaluation for ConnectGroupJoinRoute (7 gates) */
export function evaluateConnectGroupAdmission(
  ctx: ConnectGroupContext
): AdmissionDecision {
  const gateResults: AdmissionGateResult[] = [
    { gate: "RouteEnabledGate", passed: ctx.routeEnabled },
    { gate: "ConsentGate", passed: ctx.hasConsent, reason: ctx.hasConsent ? undefined : "consent not given" },
    { gate: "RoleGate", passed: ctx.hasRole },
    { gate: "CapacityGate", passed: ctx.groupHasCapacity, reason: ctx.groupHasCapacity ? undefined : "group at capacity — waitlist" },
    { gate: "ScheduleGate", passed: ctx.scheduleMatches },
    { gate: "PolicyGate", passed: !ctx.isPrivateGroup, reason: ctx.isPrivateGroup ? "private group — human approval required" : undefined },
    { gate: "NotificationBudgetGate", passed: ctx.withinNotificationBudget },
  ];

  const decision = evaluateAdmission(gateResults);

  // Private group → A3 (human required), not A4 (refuse)
  if (!decision.allowed && decision.refusalGate === "PolicyGate") {
    return { ...decision, actionClass: A3_HUMAN };
  }

  return decision;
}

/** Admin surface: which gate fails most often */
export type AdmissionGateStats = Record<AdmissionGateCode, number>;
