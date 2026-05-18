-- ZOE LA OCEL event log — stores OCEL 2.0 events for wasm4pm ingestion
-- Route: ConnectGroupJoinRoute
-- Event types: cg.interest.submitted, cg.groups.matched, cg.invite.sent,
--              cg.invite.accepted, cg.spot.reserved, cg.attendance.recorded,
--              cg.followup.created, cg.route.closed
--
-- NOTE: Hand-authored DDL — protected_path guards from ggen overwrite.
-- See: ggen.toml protected_paths entry for this file.

CREATE TABLE IF NOT EXISTS ocel_events (
  id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  event_type   TEXT        NOT NULL,           -- e.g. 'cg.invite.sent'
  object_id    TEXT        NOT NULL,           -- person_id or group_id
  object_type  TEXT        NOT NULL,           -- 'Person' or 'ConnectGroup'
  route_id     TEXT        NOT NULL,           -- 'ConnectGroupJoinRoute'
  stage_code   TEXT        NOT NULL,           -- e.g. 'invite_sent'
  action_class TEXT        NOT NULL,           -- A0-A4
  run_id       UUID        NOT NULL,           -- wasm4pm run.id
  ts_ns        BIGINT      NOT NULL,           -- timestamp in nanoseconds
  fields       JSONB       DEFAULT '{}'::jsonb, -- extra attributes
  receipt_hash TEXT,                           -- SHA-256 hex digest (Web Crypto stub)
  created_at   TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_ocel_events_run_id    ON ocel_events(run_id);
CREATE INDEX IF NOT EXISTS idx_ocel_events_route_id  ON ocel_events(route_id);
CREATE INDEX IF NOT EXISTS idx_ocel_events_object_id ON ocel_events(object_id);

ALTER TABLE ocel_events ENABLE ROW LEVEL SECURITY;

CREATE POLICY "ocel_events_read"
  ON ocel_events FOR SELECT USING (true);
