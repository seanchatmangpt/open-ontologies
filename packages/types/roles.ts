
// Generated from ontology/zoela/roles.ttl — DO NOT EDIT
// Regenerate with: ggen sync --rule zoela-roles
//
// Role permission constants for ZOE LA Mobile access control
// Source: ontology/zoela/roles.ttl → extract-role-requirements.rq → role-access.tera

export interface ZoeRole {
  code: string;
  label: string;
  permissionLevel: number;
  canViewSensitive: boolean;
  canCreateReceipts: boolean;
  canAssignVolunteers: boolean;
  requiresTraining: boolean;
  requiresBackgroundCheck: boolean;
}

export const ZOE_ROLES: Record<string, ZoeRole> = {

  'MEMBER': {
    code: 'MEMBER',
    label: 'Member',
    permissionLevel: 1,
    canViewSensitive: false,
    canCreateReceipts: false,
    canAssignVolunteers: false,
    requiresTraining: false,
    requiresBackgroundCheck: false,
  },

  'VOL': {
    code: 'VOL',
    label: 'Volunteer',
    permissionLevel: 2,
    canViewSensitive: false,
    canCreateReceipts: false,
    canAssignVolunteers: false,
    requiresTraining: true,
    requiresBackgroundCheck: false,
  },

  'PARTNER': {
    code: 'PARTNER',
    label: 'Partner Organization Rep',
    permissionLevel: 2,
    canViewSensitive: false,
    canCreateReceipts: false,
    canAssignVolunteers: false,
    requiresTraining: false,
    requiresBackgroundCheck: false,
  },

  'CARE': {
    code: 'CARE',
    label: 'Care Team Member',
    permissionLevel: 3,
    canViewSensitive: true,
    canCreateReceipts: false,
    canAssignVolunteers: false,
    requiresTraining: false,
    requiresBackgroundCheck: true,
  },

  'LEADER': {
    code: 'LEADER',
    label: 'Ministry Leader',
    permissionLevel: 3,
    canViewSensitive: true,
    canCreateReceipts: false,
    canAssignVolunteers: true,
    requiresTraining: false,
    requiresBackgroundCheck: false,
  },

  'YTH-LEAD': {
    code: 'YTH-LEAD',
    label: 'Youth Leader',
    permissionLevel: 3,
    canViewSensitive: true,
    canCreateReceipts: false,
    canAssignVolunteers: false,
    requiresTraining: true,
    requiresBackgroundCheck: true,
  },

  'ADMIN': {
    code: 'ADMIN',
    label: 'Administrator',
    permissionLevel: 4,
    canViewSensitive: true,
    canCreateReceipts: true,
    canAssignVolunteers: true,
    requiresTraining: false,
    requiresBackgroundCheck: false,
  },

  'PASTOR': {
    code: 'PASTOR',
    label: 'Pastor',
    permissionLevel: 5,
    canViewSensitive: true,
    canCreateReceipts: true,
    canAssignVolunteers: true,
    requiresTraining: false,
    requiresBackgroundCheck: false,
  },

};

export function hasRequiredRole(actorRole: string, requiredRole: string): boolean {
  const actor = ZOE_ROLES[actorRole];
  const required = ZOE_ROLES[requiredRole];
  if (!actor || !required) return false;
  return actor.permissionLevel >= required.permissionLevel;
}

export function canPerform(roleCode: string, capability: keyof ZoeRole): boolean {
  const role = ZOE_ROLES[roleCode];
  if (!role) return false;
  return role[capability] as boolean;
}
