
// Generated from ontology/zoela/routes.ttl — DO NOT EDIT
// Regenerate with: ggen sync --rule zoela-assignment-eligibility
//
// Volunteer assignment eligibility rules for ZOE LA Mobile
// Source: ontology/zoela/routes.ttl → extract-assignment-eligibility.rq → assignment-eligibility.tera

export interface AssignmentRule {
  routeName: string;
  minimumHours: number;
  requiresBackgroundCheck: boolean;
  requiredSkills: string[];
  eligibleRoles: string[];
}

export const ASSIGNMENT_RULES: AssignmentRule[] = [

  {
    routeName: "Food Distribution Route",
    minimumHours: 2,
    requiresBackgroundCheck: true,
    requiredSkills: ["Food Safety"],
    eligibleRoles: ["VOL", "CARE", "ADMIN", "PASTOR"],
  },

  {
    routeName: "Care and Pastoral Route",
    minimumHours: 4,
    requiresBackgroundCheck: true,
    requiredSkills: ["First Aid"],
    eligibleRoles: ["CARE", "LEADER", "ADMIN", "PASTOR"],
  },

  {
    routeName: "Clothing Distribution Route",
    minimumHours: 2,
    requiresBackgroundCheck: true,
    requiredSkills: ["Food Safety"],
    eligibleRoles: ["VOL", "CARE", "ADMIN", "PASTOR"],
  },

  {
    routeName: "Volunteer Assignment Route",
    minimumHours: 0,
    requiresBackgroundCheck: true,
    requiredSkills: [],
    eligibleRoles: ["LEADER", "ADMIN", "PASTOR"],
  },

  {
    routeName: "Event Registration Route",
    minimumHours: 1,
    requiresBackgroundCheck: false,
    requiredSkills: [],
    eligibleRoles: ["VOL", "LEADER", "CARE", "ADMIN", "PASTOR"],
  },

  {
    routeName: "Kids Check-In Route",
    minimumHours: 2,
    requiresBackgroundCheck: true,
    requiredSkills: ["Child Safety"],
    eligibleRoles: ["YTH-LEAD", "ADMIN", "PASTOR"],
  },

  {
    routeName: "Youth Event Participation Route",
    minimumHours: 2,
    requiresBackgroundCheck: true,
    requiredSkills: ["Child Safety"],
    eligibleRoles: ["YTH-LEAD", "LEADER", "ADMIN", "PASTOR"],
  },

  {
    routeName: "Camp Consent and Registration Route",
    minimumHours: 2,
    requiresBackgroundCheck: true,
    requiredSkills: ["Child Safety"],
    eligibleRoles: ["YTH-LEAD", "ADMIN", "PASTOR"],
  },

  {
    routeName: "Baptism Registration Route",
    minimumHours: 1,
    requiresBackgroundCheck: false,
    requiredSkills: [],
    eligibleRoles: ["LEADER", "CARE", "ADMIN", "PASTOR"],
  },

  {
    routeName: "Connect Group Join Route",
    minimumHours: 1,
    requiresBackgroundCheck: false,
    requiredSkills: [],
    eligibleRoles: ["LEADER", "ADMIN", "PASTOR"],
  },

  {
    routeName: "Partner Organization Referral Route",
    minimumHours: 2,
    requiresBackgroundCheck: true,
    requiredSkills: ["Case Management"],
    eligibleRoles: ["CARE", "ADMIN", "PASTOR"],
  },

  {
    routeName: "Care Follow-Up Route",
    minimumHours: 2,
    requiresBackgroundCheck: true,
    requiredSkills: ["First Aid"],
    eligibleRoles: ["CARE", "LEADER", "ADMIN", "PASTOR"],
  },

  {
    routeName: "Mission Trip Participation Route",
    minimumHours: 4,
    requiresBackgroundCheck: true,
    requiredSkills: ["First Aid"],
    eligibleRoles: ["VOL", "LEADER", "ADMIN", "PASTOR"],
  },

];

export function isEligible(roleCode: string, routeName: string): boolean {
  const rule = ASSIGNMENT_RULES.find(r => r.routeName === routeName);
  if (!rule) return false;
  return rule.eligibleRoles.includes(roleCode);
}

export function getRouteRule(routeName: string): AssignmentRule | undefined {
  return ASSIGNMENT_RULES.find(r => r.routeName === routeName);
}
