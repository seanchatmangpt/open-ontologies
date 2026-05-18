
-- Generated from ontology/zoela/*.ttl — DO NOT EDIT
-- Regenerate with: ggen sync --rule zoela-bridge-tables
--
-- NOTE: ggen SPARQL query currently produces no rows — bridge table DDL is hand-authored.
-- The extract-bridge-tables.rq query targets <https://zoela.org/ontology/> namespace
-- but none of the zoela TTL files consistently declare bridge classes under that namespace.

-- ============================================================================
-- Bridge table: group_memberships
-- Links persons to connect_groups (many-to-many with role + active status)
-- Source: ontology/zoela/connect-groups.ttl (zoe:GroupMembership)
-- ============================================================================
CREATE TABLE IF NOT EXISTS group_memberships (
  id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  person_id   UUID        NOT NULL REFERENCES persons(id),
  group_id    UUID        NOT NULL REFERENCES connect_groups(id),
  member_role TEXT        NOT NULL DEFAULT 'member',
  joined_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  is_active   BOOLEAN     NOT NULL DEFAULT true,
  UNIQUE (person_id, group_id)
);

ALTER TABLE group_memberships ENABLE ROW LEVEL SECURITY;

CREATE TRIGGER set_group_memberships_updated_at
  BEFORE UPDATE ON group_memberships
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE POLICY "authenticated_read_group_memberships"
  ON group_memberships FOR SELECT TO authenticated USING (true);

CREATE POLICY "authenticated_insert_group_memberships"
  ON group_memberships FOR INSERT TO authenticated WITH CHECK (true);

CREATE POLICY "owner_update_group_memberships"
  ON group_memberships FOR UPDATE TO authenticated USING (true);

-- ============================================================================
-- Bridge table: group_invites
-- Tracks invitations sent to persons to join a connect group after matching
-- Source: ontology/zoela/connect-group-routes.ttl (zoe:GroupInvite)
-- ============================================================================
CREATE TABLE IF NOT EXISTS group_invites (
  id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  person_id    UUID        NOT NULL REFERENCES persons(id),
  group_id     UUID        NOT NULL REFERENCES connect_groups(id),
  invited_by   UUID        REFERENCES persons(id),
  status       TEXT        NOT NULL DEFAULT 'pending',
  sent_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  responded_at TIMESTAMPTZ,
  UNIQUE (person_id, group_id)
);

ALTER TABLE group_invites ENABLE ROW LEVEL SECURITY;

CREATE TRIGGER set_group_invites_updated_at
  BEFORE UPDATE ON group_invites
  FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE POLICY "authenticated_read_group_invites"
  ON group_invites FOR SELECT TO authenticated USING (true);

CREATE POLICY "authenticated_insert_group_invites"
  ON group_invites FOR INSERT TO authenticated WITH CHECK (true);

CREATE POLICY "owner_update_group_invites"
  ON group_invites FOR UPDATE TO authenticated USING (true);
