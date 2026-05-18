
// Generated from ontology/zoela/*.ttl — DO NOT EDIT
// Regenerate with: ggen sync --rule zoela-ocel
//
// OCEL 2.0 event emitters for ZOE LA Mobile

export interface ZoeOcelEvent {
  eventId: string;
  eventType: string;
  timestamp: string;
  actorId: string;
  objectRefs: Array<{ objectId: string; objectType: string; relation: string }>;
  routeId?: string;
  routeStageCode?: string;
  campusId?: string;
  ministryCode?: string;
  receiptId?: string;
  evidenceId?: string;
  outcomeState?: string;
}


