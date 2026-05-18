// Care type stubs — expanded by ggen pipeline as care.ttl grows
// Source: ontology/zoela/care.ttl

export interface FollowUpRule {
  label: string;
  triggerCondition: string;
  followUpAction: string;
  daysAfter: number;
  assignedRoles: string[];
}

export interface CareRecord {
  id: string;
  personId: string;
  careType: string;
  status: string;
  assignedTo?: string;
  createdAt: string;
}
