// Connect Group Join Route — POWL v2 partial-order descriptor
// Compatible with mcpp powl_bridge.rs and powl-v2.schema.json
// Source: ontology/zoela/connect-group-routes.ttl — zoe:predecessorStage edges
//
// Field vocabulary mirrors mcpp schemas/routes/powl-v2.schema.json:
//   route_id, type: "powl2", required_stages, activities (id/label/object_type/description),
//   edges (from/to), object_types (created_by/terminated_by/min_count),
//   receipt_required, model (choice_graph), gap_closure_authority
//
// PowlAdmissionEvidence in mcpp-core/src/powl_bridge.rs consumes:
//   output_hash, input_hash, plan_hash, fitness (must == 1.0), precision (must == 1.0)

// ─── Activity node ─────────────────────────────────────────────────────────
// Maps to mcpp Activity ($defs/Activity in powl-v2.schema.json)
export interface PowlActivity {
  /** Stage code — lowercase with hyphens, matches required_stages entries. */
  id: string;
  /** Human-readable label surfaced in launch-report.md and AAT orchestrator. */
  label: string;
  /** OCEL 2.0 object type created or updated by this activity. */
  object_type: string;
  /** Emitted on completion — recorded in the OCEL event log. */
  ocel_event_type: string;
  /** Autonomic action class (A0-A4). Determines execution authority. */
  autonomic_class: string;
  /** Whether this activity is the route entry point. */
  is_entry: boolean;
  /** Whether this activity is the route terminal (Receipt must be emitted here). */
  is_terminal: boolean;
  /** Free-form description surfaced in mcpp-lsp diagnostics. */
  description: string;
  /** Receipt required at this stage (POWL-R03). */
  receipt_required: boolean;
}

// ─── Precedence edge ───────────────────────────────────────────────────────
// Maps to mcpp Edge ($defs/Edge in powl-v2.schema.json)
export interface PowlEdge {
  /** Predecessor activity id. */
  from: string;
  /** Successor activity id. */
  to: string;
}

// ─── Object type spec ──────────────────────────────────────────────────────
// Maps to mcpp ObjectTypeSpec ($defs/ObjectTypeSpec in powl-v2.schema.json)
export interface PowlObjectTypeSpec {
  created_by?: string[];
  terminated_by?: string[];
  min_count?: number;
  schema?: string;
}

// ─── Gap closure authority entry ───────────────────────────────────────────
// Maps to mcpp GapAuthority ($defs/GapAuthority in powl-v2.schema.json)
// Z-P09 gap-closing protocol — declares which alternate evidence sources may
// close a gap for a given activity when primary evidence is unavailable.
export interface PowlGapAuthority {
  activity_id: string;
  alternate_evidence_sources: string[];
  max_retries?: number;
  exhaustion_refusal_class?: string;
}

// ─── POWL v2 route descriptor ──────────────────────────────────────────────
// Root object compatible with mcpp powl-v2.schema.json
export interface PowlDescriptor {
  /** Stable route identifier — stamped on every receipt. */
  route_id: string;
  /** Schema tag — mcpp v26.5.18+ accepts only "powl2". */
  type: "powl2";
  /** Schema version tag. */
  schema_version: string;
  /** Human description surfaced in launch-report.md. */
  description: string;
  /**
   * Ordered stage IDs that must appear as activity or choice_graph nodes.
   * POWL-R01: every entry here must be present in activities[].id.
   */
  required_stages: string[];
  /** Per-activity metadata consumed by AAT orchestrator and Erlang generation. */
  activities: PowlActivity[];
  /**
   * Plain (from, to) precedence edges — alternative to model.choice_graph.edges.
   * The 8-stage Connect Group route is a strict sequence (no fan-out/join),
   * so a choice_graph with a linear chain satisfies the partial-order requirement.
   */
  edges: PowlEdge[];
  /** Object types referenced by this route. POWL-R03 enforces lifecycle. */
  object_types: Record<string, PowlObjectTypeSpec>;
  /** POWL-R03: terminal activity must emit a Receipt object. */
  receipt_required: boolean;
  /** Structural model. "choice_graph" expresses the sequential partial order. */
  model: {
    type: "choice_graph";
    choice_graph: {
      /** ▷ = source node; □ = sink node; remaining are activity ids. */
      nodes: string[];
      /** Each entry is a [from, to] pair. */
      edges: [string, string][];
    };
  };
  /** Z-P09 gap-closing protocol for activities that may lack primary evidence. */
  gap_closure_authority: PowlGapAuthority[];
  /** Whether this route participates in cross-enterprise relay. */
  emits_relay: boolean;
  /** Autonomic law governing the route as a whole. */
  autonomic_law: string;
}

// ───────────────────────────────────────────────────────────────────────────
// Connect Group Join Route — 8-stage POWL v2 DAG
//
// Precedence chain (from ontology/zoela/connect-group-routes.ttl):
//
//   cg-interest-expressed
//     → cg-preferences-collected
//       → cg-eligible-groups-found
//         → cg-invite-sent
//           → cg-invite-accepted
//             → cg-first-meeting-attended
//               → cg-followup-completed
//                 → cg-membership-active
//
// Admission gates (zoe:requiresGate) are expressed as gap_closure_authority
// entries so the AAT orchestrator can resolve missing evidence before refusing.
// ───────────────────────────────────────────────────────────────────────────
export const CONNECT_GROUP_JOIN_ROUTE: PowlDescriptor = {
  route_id: "connect-group-join-route",
  type: "powl2",
  schema_version: "powl/v2.1",
  description:
    "ZOE LA Mobile — Connect Group Join Route. " +
    "If a person expresses interest, the system matches, invites, reminds, " +
    "waitlists, and escalates without human action when route law permits.",

  // POWL-R01: every id below must appear in activities[].id
  required_stages: [
    "cg_interest_expressed",
    "cg_preferences_collected",
    "cg_eligible_groups_found",
    "cg_invite_sent",
    "cg_invite_accepted",
    "cg_first_meeting_attended",
    "cg_followup_completed",
    "cg_membership_active",
  ],

  activities: [
    {
      id: "cg_interest_expressed",
      label: "Interest Expressed",
      object_type: "ConnectGroupInterest",
      ocel_event_type: "connect_group.interest_expressed",
      autonomic_class: "A1",
      is_entry: true,
      is_terminal: false,
      receipt_required: false,
      description:
        "Person submits connect group interest with schedule/location preferences. " +
        "Corresponds to zoe:CGStage1InterestExpressed in connect-group-routes.ttl.",
    },
    {
      id: "cg_preferences_collected",
      label: "Preferences Collected",
      object_type: "ConnectGroupPreferences",
      ocel_event_type: "connect_group.preferences_collected",
      autonomic_class: "A1",
      is_entry: false,
      is_terminal: false,
      receipt_required: false,
      description:
        "Schedule, location, and generation segment preferences captured. " +
        "Corresponds to zoe:CGStage2PreferencesCollected.",
    },
    {
      id: "cg_eligible_groups_found",
      label: "Eligible Groups Found",
      object_type: "EligibleGroupMatch",
      ocel_event_type: "connect_group.eligible_groups_found",
      autonomic_class: "A2",
      is_entry: false,
      is_terminal: false,
      receipt_required: false,
      description:
        "Autonomic matching against open groups by schedule/location/capacity. " +
        "Waitlist emitted if all groups are full. " +
        "Requires CapacityGate and ScheduleGate. " +
        "Corresponds to zoe:CGStage3EligibleGroupsFound.",
    },
    {
      id: "cg_invite_sent",
      label: "Invite Sent",
      object_type: "GroupInvite",
      ocel_event_type: "connect_group.invite_sent",
      autonomic_class: "A2",
      is_entry: false,
      is_terminal: false,
      receipt_required: false,
      description:
        "Autonomic invite sent if consent exists. Refusal emitted if ConsentGate or " +
        "NotificationBudgetGate fails. " +
        "Corresponds to zoe:CGStage4InviteSent.",
    },
    {
      id: "cg_invite_accepted",
      label: "Invite Accepted",
      object_type: "GroupInviteAcceptance",
      ocel_event_type: "connect_group.invite_accepted",
      autonomic_class: "A1",
      is_entry: false,
      is_terminal: false,
      receipt_required: false,
      description:
        "Person accepts group invite. Spot reserved. Requires CapacityGate re-check. " +
        "Corresponds to zoe:CGStage5InviteAccepted.",
    },
    {
      id: "cg_first_meeting_attended",
      label: "First Meeting Attended",
      object_type: "MeetingAttendance",
      ocel_event_type: "connect_group.first_meeting_attended",
      autonomic_class: "A1",
      is_entry: false,
      is_terminal: false,
      receipt_required: false,
      description:
        "Attendance recorded at first group meeting. Evidence artifact created. " +
        "Corresponds to zoe:CGStage6FirstMeetingAttended.",
    },
    {
      id: "cg_followup_completed",
      label: "Follow-Up Completed",
      object_type: "GroupFollowUp",
      ocel_event_type: "connect_group.followup_completed",
      autonomic_class: "A1",
      is_entry: false,
      is_terminal: false,
      receipt_required: false,
      description:
        "Follow-up completed after first meeting. Missed first meeting triggers " +
        "automatic follow-up work order. " +
        "Corresponds to zoe:CGStage7FollowUpCompleted.",
    },
    {
      id: "cg_membership_active",
      label: "Membership Active",
      object_type: "ConnectGroupMembership",
      ocel_event_type: "connect_group.membership_active",
      autonomic_class: "A1",
      is_entry: false,
      is_terminal: true,
      receipt_required: true,
      description:
        "Route complete. Receipt emitted. Person is active member of connect group. " +
        "Corresponds to zoe:CGStage8MembershipActive.",
    },
  ],

  // 7 sequential precedence edges derived from zoe:predecessorStage in TTL
  edges: [
    { from: "cg_interest_expressed",    to: "cg_preferences_collected"    },
    { from: "cg_preferences_collected", to: "cg_eligible_groups_found"    },
    { from: "cg_eligible_groups_found", to: "cg_invite_sent"              },
    { from: "cg_invite_sent",           to: "cg_invite_accepted"          },
    { from: "cg_invite_accepted",       to: "cg_first_meeting_attended"   },
    { from: "cg_first_meeting_attended",to: "cg_followup_completed"       },
    { from: "cg_followup_completed",    to: "cg_membership_active"        },
  ],

  // OCEL object lifecycle obligations (POWL-R03)
  object_types: {
    ConnectGroupInterest: {
      created_by: ["cg_interest_expressed"],
    },
    ConnectGroupPreferences: {
      created_by: ["cg_preferences_collected"],
    },
    EligibleGroupMatch: {
      created_by: ["cg_eligible_groups_found"],
    },
    GroupInvite: {
      created_by: ["cg_invite_sent"],
    },
    GroupInviteAcceptance: {
      created_by: ["cg_invite_accepted"],
    },
    MeetingAttendance: {
      created_by: ["cg_first_meeting_attended"],
    },
    GroupFollowUp: {
      created_by: ["cg_followup_completed"],
    },
    ConnectGroupMembership: {
      created_by: ["cg_membership_active"],
      min_count: 1,
    },
    Receipt: {
      created_by: ["cg_membership_active"],
      schema: "schemas/receipts/proof-receipt.schema.json",
      min_count: 1,
    },
  },

  // POWL-R03: terminal activity emits a Receipt
  receipt_required: true,

  // Structural model — linear partial order expressed as choice_graph
  // ▷ = source (POWL start token), □ = sink (POWL end token)
  model: {
    type: "choice_graph",
    choice_graph: {
      nodes: [
        "▷",
        "cg_interest_expressed",
        "cg_preferences_collected",
        "cg_eligible_groups_found",
        "cg_invite_sent",
        "cg_invite_accepted",
        "cg_first_meeting_attended",
        "cg_followup_completed",
        "cg_membership_active",
        "□",
      ],
      edges: [
        ["▷",                         "cg_interest_expressed"    ],
        ["cg_interest_expressed",     "cg_preferences_collected" ],
        ["cg_preferences_collected",  "cg_eligible_groups_found" ],
        ["cg_eligible_groups_found",  "cg_invite_sent"           ],
        ["cg_invite_sent",            "cg_invite_accepted"       ],
        ["cg_invite_accepted",        "cg_first_meeting_attended"],
        ["cg_first_meeting_attended", "cg_followup_completed"    ],
        ["cg_followup_completed",     "cg_membership_active"     ],
        ["cg_membership_active",      "□"                        ],
      ],
    },
  },

  // Z-P09 gap-closure: activities with required gates declare alternate
  // evidence sources the route_coordinator may consult before refusing.
  gap_closure_authority: [
    {
      // cg_eligible_groups_found requires CapacityGate + ScheduleGate
      activity_id: "cg_eligible_groups_found",
      alternate_evidence_sources: [
        "zoe:CapacityGate",
        "zoe:ScheduleGate",
        "waitlist_record",
      ],
      max_retries: 3,
      exhaustion_refusal_class: "CAPACITY_GATE_EXHAUSTED",
    },
    {
      // cg_invite_sent requires ConsentGate + NotificationBudgetGate
      activity_id: "cg_invite_sent",
      alternate_evidence_sources: [
        "zoe:ConsentGate",
        "zoe:NotificationBudgetGate",
        "consent_record",
      ],
      max_retries: 1,
      exhaustion_refusal_class: "CONSENT_GATE_REFUSED",
    },
    {
      // cg_invite_accepted requires CapacityGate re-check at spot reservation
      activity_id: "cg_invite_accepted",
      alternate_evidence_sources: [
        "zoe:CapacityGate",
        "spot_reservation_record",
      ],
      max_retries: 2,
      exhaustion_refusal_class: "SPOT_CAPACITY_EXHAUSTED",
    },
  ],

  emits_relay: false,

  autonomic_law:
    "If person expresses interest → system matches/invites/reminds/waitlists/" +
    "escalates without human action when route law permits.",
};
