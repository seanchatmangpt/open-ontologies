// Resource type stubs — expanded by ggen pipeline as resource.ttl grows
// Source: ontology/zoela/resource.ttl

export interface ResourceMatch {
  label: string;
  type: string;
  capacity: number;
  eligibilityCriteria: string;
  applicableRoutes: string[];
}

export interface Resource {
  id: string;
  code: string;
  label: string;
  type: string;
  campusId?: string;
  available: boolean;
}
