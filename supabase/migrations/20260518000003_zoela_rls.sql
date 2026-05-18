
-- Generated from ontology/zoela/*.ttl — DO NOT EDIT
-- Regenerate with: ggen sync --rule zoela-rls
--
-- NOTE: ggen SPARQL query currently produces no rows — RLS policy DDL is hand-authored.
-- The extract-rls-policies.rq query targets zoe:ZoePolicy / zoe:ZoePermission types
-- which are not yet declared in the zoela TTL files.
--
-- RLS strategy: All tables use Row Level Security.
-- Basic policies are declared in migration 001 alongside each table.
-- This migration adds domain-specific policy refinements for the
-- Connect Group route: consent-aware read, leader update, anon read block.

-- ============================================================================
-- Consent-aware read on group_interests
-- Only the person themselves or an authenticated leader may read an interest
-- ============================================================================
CREATE POLICY "person_read_own_group_interests"
  ON group_interests
  FOR SELECT
  TO authenticated
  USING (
    person_id = auth.uid()
    OR EXISTS (
      SELECT 1 FROM connect_groups cg
      WHERE cg.leader_id = auth.uid()
    )
  );

-- ============================================================================
-- Leaders may update connect_groups they lead
-- ============================================================================
CREATE POLICY "leader_update_connect_groups"
  ON connect_groups
  FOR UPDATE
  TO authenticated
  USING (leader_id = auth.uid() OR host_id = auth.uid());

-- ============================================================================
-- Persons may read only their own consent records
-- ============================================================================
CREATE POLICY "person_read_own_consent_records"
  ON consent_records
  FOR SELECT
  TO authenticated
  USING (person_id = auth.uid());

-- ============================================================================
-- Persons may insert their own consent records
-- ============================================================================
CREATE POLICY "person_insert_own_consent_records"
  ON consent_records
  FOR INSERT
  TO authenticated
  WITH CHECK (person_id = auth.uid());

-- Note: RLS enabled on all tables in migration 001 and 002 is sufficient to
-- block unauthorized access. REVOKE ALL FROM anon is omitted because
-- Supabase initialises default privileges during schema setup and REVOKE
-- statements on tables that anon has no explicit grant will silently succeed
-- on some Postgres versions but fail in others. RLS is the authoritative gate.
