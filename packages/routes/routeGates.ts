
// Generated from ontology/zoela/*.ttl — DO NOT EDIT
// Regenerate with: ggen sync --rule zoela-service-routes
//
// Route stage gate functions for ZOE LA Mobile
// Source: ontology/zoela/routes.ttl → extract-service-routes.rq → route-stage-gate.tera

export interface RouteStageGate {
  routeCode: string;
  stageCode: string;
  stageLabel: string;
  stageOrder: number;
  isEntry: boolean;
  isTerminal: boolean;
  predecessorCode?: string;
  requiredRoleCode?: string;
  requiredConsentCode?: string;
  completionReceiptType?: string;
  ocelEventType?: string;
  timeoutHours?: number;
}

export const ROUTE_STAGE_GATES: RouteStageGate[] = [

  {
    routeCode: 'FOOD_DIST_V1',
    stageCode: 'received',
    stageLabel: 'Received',
    stageOrder: 0,
    isEntry: true,
    isTerminal: false,
    
    requiredRoleCode: 'MemberRole',
    
    
    ocelEventType: 'food_route.received',
    
  },

  {
    routeCode: 'FOOD_DIST_V1',
    stageCode: 'verified',
    stageLabel: 'Verified',
    stageOrder: 1,
    isEntry: false,
    isTerminal: false,
    predecessorCode: 'received',
    requiredRoleCode: 'VolunteerRole',
    
    
    ocelEventType: 'food_route.verified',
    
  },

  {
    routeCode: 'FOOD_DIST_V1',
    stageCode: 'assigned',
    stageLabel: 'Assigned',
    stageOrder: 2,
    isEntry: false,
    isTerminal: false,
    predecessorCode: 'verified',
    requiredRoleCode: 'VolunteerRole',
    
    
    ocelEventType: 'food_route.assigned',
    
  },

  {
    routeCode: 'FOOD_DIST_V1',
    stageCode: 'delivered',
    stageLabel: 'Delivered',
    stageOrder: 3,
    isEntry: false,
    isTerminal: false,
    predecessorCode: 'assigned',
    requiredRoleCode: 'VolunteerRole',
    
    completionReceiptType: 'ResourceDistributedReceipt',
    ocelEventType: 'food.delivered',
    
  },

  {
    routeCode: 'FOOD_DIST_V1',
    stageCode: 'closed',
    stageLabel: 'Closed',
    stageOrder: 4,
    isEntry: false,
    isTerminal: true,
    predecessorCode: 'delivered',
    
    
    completionReceiptType: 'CareRouteClosedReceipt',
    ocelEventType: 'care_route.closed',
    
  },

];

export function getStageGate(routeCode: string, stageCode: string): RouteStageGate | undefined {
  return ROUTE_STAGE_GATES.find(g => g.routeCode === routeCode && g.stageCode === stageCode);
}

export function getEntryStage(routeCode: string): RouteStageGate | undefined {
  return ROUTE_STAGE_GATES.find(g => g.routeCode === routeCode && g.isEntry);
}

export function getNextStages(routeCode: string, currentStageCode: string): RouteStageGate[] {
  return ROUTE_STAGE_GATES.filter(
    g => g.routeCode === routeCode && g.predecessorCode === currentStageCode
  );
}

export function isGateBlocked(gate: RouteStageGate, context: {
  actorRoleCode?: string;
  grantedConsentCodes?: string[];
}): { blocked: boolean; reason?: string } {
  if (gate.requiredRoleCode && context.actorRoleCode !== gate.requiredRoleCode) {
    return { blocked: true, reason: `Requires role: ${gate.requiredRoleCode}` };
  }
  if (gate.requiredConsentCode && !context.grantedConsentCodes?.includes(gate.requiredConsentCode)) {
    return { blocked: true, reason: `Missing consent: ${gate.requiredConsentCode}` };
  }
  return { blocked: false };
}
