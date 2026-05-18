// Route type stubs — expanded by ggen pipeline as route ontology grows
// Source: ontology/zoela/connect-group-routes.ttl

export interface RouteStage {
  code: string;
  label: string;
  order: number;
  isEntry: boolean;
  isTerminal: boolean;
  predecessorCode: string;
  ocelEventType: string;
  autonomicActionClass: string;
  requiredGates: string[];
}

export interface WorkOrder {
  code: string;
  label: string;
  mappedStageCode: string;
  autonomicActionClass: string;
  requiresHumanApproval: boolean;
}

export interface AdminSurface {
  code: string;
  label: string;
  urgencyLevel: string;
  description: string;
  relatedStage?: string;
}

export interface RouteGate {
  routeCode: string;
  stageCode: string;
  gateCode: string;
  gateClass: string;
}
