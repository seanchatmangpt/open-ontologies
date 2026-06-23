
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



// Interest Submitted
// OCEL event type: cg.interest.submitted
// Member submits interest in joining a connect group
// Emitted by route: connect-group
export function emitCgInterestSubmittedEvent(
  params: Omit<ZoeOcelEvent, 'eventId' | 'timestamp' | 'eventType'>
): ZoeOcelEvent {
  return {
    ...params,
    eventId: crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    eventType: 'cg.interest.submitted',
  };
}

// Groups Matched
// OCEL event type: cg.groups.matched
// System matches member to eligible connect groups
// Emitted by route: connect-group
export function emitCgGroupsMatchedEvent(
  params: Omit<ZoeOcelEvent, 'eventId' | 'timestamp' | 'eventType'>
): ZoeOcelEvent {
  return {
    ...params,
    eventId: crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    eventType: 'cg.groups.matched',
  };
}

// Invite Sent
// OCEL event type: cg.invite.sent
// Connect group leader sends invitation to member
// Emitted by route: connect-group
export function emitCgInviteSentEvent(
  params: Omit<ZoeOcelEvent, 'eventId' | 'timestamp' | 'eventType'>
): ZoeOcelEvent {
  return {
    ...params,
    eventId: crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    eventType: 'cg.invite.sent',
  };
}

// Invite Accepted
// OCEL event type: cg.invite.accepted
// Member accepts connect group invitation
// Emitted by route: connect-group
export function emitCgInviteAcceptedEvent(
  params: Omit<ZoeOcelEvent, 'eventId' | 'timestamp' | 'eventType'>
): ZoeOcelEvent {
  return {
    ...params,
    eventId: crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    eventType: 'cg.invite.accepted',
  };
}

// Spot Reserved
// OCEL event type: cg.spot.reserved
// Member spot reserved in connect group
// Emitted by route: connect-group
export function emitCgSpotReservedEvent(
  params: Omit<ZoeOcelEvent, 'eventId' | 'timestamp' | 'eventType'>
): ZoeOcelEvent {
  return {
    ...params,
    eventId: crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    eventType: 'cg.spot.reserved',
  };
}

// Attendance Recorded
// OCEL event type: cg.attendance.recorded
// Attendance recorded for connect group session
// Emitted by route: connect-group
export function emitCgAttendanceRecordedEvent(
  params: Omit<ZoeOcelEvent, 'eventId' | 'timestamp' | 'eventType'>
): ZoeOcelEvent {
  return {
    ...params,
    eventId: crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    eventType: 'cg.attendance.recorded',
  };
}

// Follow-up Created
// OCEL event type: cg.followup.created
// Pastoral follow-up created for member engagement
// Emitted by route: connect-group
export function emitCgFollowupCreatedEvent(
  params: Omit<ZoeOcelEvent, 'eventId' | 'timestamp' | 'eventType'>
): ZoeOcelEvent {
  return {
    ...params,
    eventId: crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    eventType: 'cg.followup.created',
  };
}

// Route Closed
// OCEL event type: cg.route.closed
// Connect group route completed and closed
// Emitted by route: connect-group
export function emitCgRouteClosedEvent(
  params: Omit<ZoeOcelEvent, 'eventId' | 'timestamp' | 'eventType'>
): ZoeOcelEvent {
  return {
    ...params,
    eventId: crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    eventType: 'cg.route.closed',
  };
}

// Food Route Received
// OCEL event type: food_route.received
// Food Route Received
// Emitted by route: food-route
export function emitFoodRouteReceivedEvent(
  params: Omit<ZoeOcelEvent, 'eventId' | 'timestamp' | 'eventType'>
): ZoeOcelEvent {
  return {
    ...params,
    eventId: crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    eventType: 'food_route.received',
  };
}

// Food Route Verified
// OCEL event type: food_route.verified
// Verification stage: a volunteer confirms the details
// Emitted by route: food-route
export function emitFoodRouteVerifiedEvent(
  params: Omit<ZoeOcelEvent, 'eventId' | 'timestamp' | 'eventType'>
): ZoeOcelEvent {
  return {
    ...params,
    eventId: crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    eventType: 'food_route.verified',
  };
}

// Food Route Assigned
// OCEL event type: food_route.assigned
// Assignment stage: a volunteer is matched to delivery
// Emitted by route: food-route
export function emitFoodRouteAssignedEvent(
  params: Omit<ZoeOcelEvent, 'eventId' | 'timestamp' | 'eventType'>
): ZoeOcelEvent {
  return {
    ...params,
    eventId: crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    eventType: 'food_route.assigned',
  };
}

// Food Delivered
// OCEL event type: food.delivered
// A food resource was delivered to recipient
// Emitted by route: food-route
export function emitFoodDeliveredEvent(
  params: Omit<ZoeOcelEvent, 'eventId' | 'timestamp' | 'eventType'>
): ZoeOcelEvent {
  return {
    ...params,
    eventId: crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    eventType: 'food.delivered',
  };
}

// Care Route Closed
// OCEL event type: care_route.closed
// A care route instance was closed after all stages
// Emitted by route: care-route
export function emitCareRouteClosedEvent(
  params: Omit<ZoeOcelEvent, 'eventId' | 'timestamp' | 'eventType'>
): ZoeOcelEvent {
  return {
    ...params,
    eventId: crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    eventType: 'care_route.closed',
  };
}