
// Generated from ontology/zoela/*.ttl — DO NOT EDIT
// Regenerate with: ggen sync --target zoela-mobile
//
// Architecture:
//   Tier 1 — ontology/zoela/*.ttl           (source of truth)
//   Tier 2 — ggen sync                      (manufacturing step)
//   Tier 3 — src/types/zoela.ts             (this file, imported by app)

// ============================================================================
// Shared base fields injected into every generated interface
// ============================================================================
export interface ZoelaBase {
  /** Surrogate primary key (UUID v4) */
  id: string;
  /** Row creation timestamp (ISO-8601) */
  created_at: string;
  /** Row last-update timestamp (ISO-8601) — set by Supabase trigger */
  updated_at?: string;
}

// ============================================================================
// Domain interfaces — one per OWL class in ontology/zoela/*.ttl
// ============================================================================


// ============================================================================
// Enum types from SKOS ConceptSchemes in ontology/zoela/*.ttl
// ============================================================================

