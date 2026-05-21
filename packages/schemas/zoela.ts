
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


// ----------------------------------------------------------------------------
// Contribution Unit Shape
// ----------------------------------------------------------------------------
export const contribution unit Schema = zoelaBaseSchema.extend({
  // 
  targetRepository: z.string(),
});

export type Contribution Unit  = z.infer<typeof contribution unit Schema>;

// ----------------------------------------------------------------------------
// No Synthetic Closure Shape
// ----------------------------------------------------------------------------
export const no synthetic closure Schema = zoelaBaseSchema.extend({
  // 
  refusalState: z.string().optional(),
});

export type No Synthetic Closure  = z.infer<typeof no synthetic closure Schema>;

// ----------------------------------------------------------------------------
// Contribution Receipt Shape
// ----------------------------------------------------------------------------
export const contribution receipt Schema = zoelaBaseSchema.extend({
  // 
  refusalState: z.string().optional(),
  // 
  expectedClosureHash: z.string(),
  // 
  observedEvidenceHash: z.string().regex(/^[a-f0-9]{64}$/),
  // 
  wasGeneratedBy: z.string(),
});

export type Contribution Receipt  = z.infer<typeof contribution receipt Schema>;

// ============================================================================
// Insert schemas — strip id/created_at/updated_at (Supabase generates these)
// ============================================================================

export const contribution unit InsertSchema =
  contribution unit Schema.omit({ id: true, created_at: true, updated_at: true });

export type Contribution Unit Insert =
  z.infer<typeof contribution unit InsertSchema>;

export const no synthetic closure InsertSchema =
  no synthetic closure Schema.omit({ id: true, created_at: true, updated_at: true });

export type No Synthetic Closure Insert =
  z.infer<typeof no synthetic closure InsertSchema>;

export const contribution receipt InsertSchema =
  contribution receipt Schema.omit({ id: true, created_at: true, updated_at: true });

export type Contribution Receipt Insert =
  z.infer<typeof contribution receipt InsertSchema>;

