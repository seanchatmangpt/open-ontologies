
// Generated from SHACL shapes in ontology/zoela/*.ttl — DO NOT EDIT
// Regenerate with: ggen sync --target zoela-mobile
//
// Architecture:
//   Tier 1 — ontology/zoela/*.ttl           (source of truth — SHACL shapes)
//   Tier 2 — ggen sync                      (manufacturing step)
//   Tier 3 — src/schemas/zoela.ts           (this file, used by app forms)
import { z } from 'zod';

// ============================================================================
// Shared base schema — applied via .merge() or .extend() in each domain schema
// ============================================================================
export const zoelaBaseSchema = z.object({
  id:         z.string().uuid(),
  created_at: z.string().datetime({ offset: true }),
  updated_at: z.string().datetime({ offset: true }).optional(),
});

// ============================================================================
// Domain schemas — one per SHACL NodeShape targeting a ZOE LA class
// ============================================================================


// ============================================================================
// Insert schemas — strip id/created_at/updated_at (Supabase generates these)
// ============================================================================


