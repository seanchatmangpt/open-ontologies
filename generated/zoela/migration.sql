
-- Generated from ontology/zoela/*.ttl — DO NOT EDIT
-- Regenerate with: ggen sync --target zoela-mobile
--
-- Architecture:
--   Tier 1 — ontology/zoela/*.ttl                (source of truth)
--   Tier 2 — ggen sync                           (manufacturing step)
--   Tier 3 — supabase/migrations/zoela_init.sql  (this file, applied by CLI)

-- ============================================================================
-- Shared helper: updated_at trigger function (idempotent)
-- ============================================================================
CREATE OR REPLACE FUNCTION set_updated_at()
  RETURNS TRIGGER
  LANGUAGE plpgsql
AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$;


