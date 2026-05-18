
-- Generated from ontology/zoela/*.ttl — DO NOT EDIT
-- Regenerate with: ggen sync --target zoela-mobile
--
-- Architecture:
--   Tier 1 — ontology/zoela/*.ttl                (source of truth)
--   Tier 2 — ggen sync                           (manufacturing step)
--   Tier 3 — supabase/migrations/zoela_init.sql  (this file, applied by CLI)
--
-- NOTE: ggen SPARQL queries currently produce no rows due to namespace mismatch
-- between TTL files. DDL is hand-authored here and protected_paths guards it
-- from being overwritten until the namespace inconsistency is resolved.
-- See: .specify/queries/zoela/extract-tables.rq (expects <urn:zoela:> prefix)
--      ontology/zoela/person.ttl     (uses <urn:zoela:>)
--      ontology/zoela/campus.ttl     (uses <https://zoela.org/onto/>)
--      ontology/zoela/connect-groups.ttl (uses <https://zoela.org/onto/>)

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

-- ============================================================================
-- Table: campuses
-- Source: ontology/zoela/campus.ttl (zoe:Campus)
-- A physical church campus operated by ZOE LA.
-- ============================================================================

CREATE TABLE IF NOT EXISTS campuses (
  id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at     TIMESTAMPTZ,
  name           TEXT        NOT NULL,
  slug           TEXT        NOT NULL UNIQUE,
  campus_code    TEXT,
  campus_city    TEXT,
  location_mode  TEXT        NOT NULL DEFAULT 'in-person',
  campus_timezone TEXT       DEFAULT 'America/Los_Angeles',
  is_active      BOOLEAN     NOT NULL DEFAULT true
);

ALTER TABLE campuses ENABLE ROW LEVEL SECURITY;

CREATE TRIGGER set_campuses_updated_at
  BEFORE UPDATE ON campuses
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE POLICY "authenticated_read_campuses"
  ON campuses FOR SELECT TO authenticated USING (true);

-- ============================================================================
-- Table: persons
-- Source: ontology/zoela/person.ttl (zoe:PersonProfile)
-- A church member or visitor tracked in the ZOE LA Mobile app.
-- ============================================================================

CREATE TABLE IF NOT EXISTS persons (
  id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at  TIMESTAMPTZ,
  full_name   TEXT        NOT NULL,
  email       TEXT,
  phone       TEXT,
  role        TEXT        DEFAULT 'member',
  campus_id   UUID        REFERENCES campuses(id)
);

ALTER TABLE persons ENABLE ROW LEVEL SECURITY;

CREATE TRIGGER set_persons_updated_at
  BEFORE UPDATE ON persons
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE POLICY "authenticated_read_persons"
  ON persons FOR SELECT TO authenticated USING (true);

CREATE POLICY "authenticated_insert_persons"
  ON persons FOR INSERT TO authenticated WITH CHECK (true);

CREATE POLICY "owner_update_persons"
  ON persons FOR UPDATE TO authenticated USING (true);

-- ============================================================================
-- Table: connect_groups
-- Source: ontology/zoela/connect-groups.ttl (zoe:ConnectGroup)
-- A small-group community formation unit within ZOE LA.
-- ============================================================================

CREATE TABLE IF NOT EXISTS connect_groups (
  id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at      TIMESTAMPTZ,
  name            TEXT        NOT NULL,
  campus_id       UUID        REFERENCES campuses(id),
  leader_id       UUID        REFERENCES persons(id),
  host_id         UUID        REFERENCES persons(id),
  group_code      TEXT,
  max_capacity    INTEGER     NOT NULL DEFAULT 12,
  current_count   INTEGER     NOT NULL DEFAULT 0,
  is_private      BOOLEAN     NOT NULL DEFAULT false,
  location_mode   TEXT        NOT NULL DEFAULT 'in-person',
  meeting_day     TEXT,
  meeting_time    TEXT,
  group_frequency TEXT        DEFAULT 'weekly',
  is_open         BOOLEAN     NOT NULL DEFAULT true
);

ALTER TABLE connect_groups ENABLE ROW LEVEL SECURITY;

CREATE TRIGGER set_connect_groups_updated_at
  BEFORE UPDATE ON connect_groups
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE POLICY "authenticated_read_connect_groups"
  ON connect_groups FOR SELECT TO authenticated USING (true);

CREATE POLICY "authenticated_insert_connect_groups"
  ON connect_groups FOR INSERT TO authenticated WITH CHECK (true);

CREATE POLICY "owner_update_connect_groups"
  ON connect_groups FOR UPDATE TO authenticated USING (true);

-- ============================================================================
-- Table: consent_records
-- Source: ontology/zoela/consent.ttl (zoe:Consent)
-- A recorded consent granted by or on behalf of a person.
-- ============================================================================

CREATE TABLE IF NOT EXISTS consent_records (
  id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at   TIMESTAMPTZ,
  person_id    UUID        NOT NULL REFERENCES persons(id),
  consent_type TEXT        NOT NULL,
  granted      BOOLEAN     NOT NULL DEFAULT false,
  granted_at   TIMESTAMPTZ,
  revoked_at   TIMESTAMPTZ,
  granted_by   TEXT,
  is_guardian_consent BOOLEAN DEFAULT false
);

ALTER TABLE consent_records ENABLE ROW LEVEL SECURITY;

CREATE TRIGGER set_consent_records_updated_at
  BEFORE UPDATE ON consent_records
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE POLICY "authenticated_read_consent_records"
  ON consent_records FOR SELECT TO authenticated USING (true);

CREATE POLICY "authenticated_insert_consent_records"
  ON consent_records FOR INSERT TO authenticated WITH CHECK (true);

-- ============================================================================
-- Table: group_interests
-- Source: ontology/zoela/connect-group-routes.ttl (zoe:GroupInterest)
-- An expression of interest by a person seeking to join a Connect Group.
-- ============================================================================

CREATE TABLE IF NOT EXISTS group_interests (
  id                    UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at            TIMESTAMPTZ,
  person_id             UUID        NOT NULL REFERENCES persons(id),
  campus_id             UUID        REFERENCES campuses(id),
  schedule_preference   TEXT,
  location_preference   TEXT        DEFAULT 'in-person',
  submitted_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  matched_group_id      UUID        REFERENCES connect_groups(id),
  match_status          TEXT        NOT NULL DEFAULT 'pending'
);

ALTER TABLE group_interests ENABLE ROW LEVEL SECURITY;

CREATE TRIGGER set_group_interests_updated_at
  BEFORE UPDATE ON group_interests
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE POLICY "authenticated_read_group_interests"
  ON group_interests FOR SELECT TO authenticated USING (true);

CREATE POLICY "authenticated_insert_group_interests"
  ON group_interests FOR INSERT TO authenticated WITH CHECK (true);

CREATE POLICY "owner_update_group_interests"
  ON group_interests FOR UPDATE TO authenticated USING (true);
