-- Connect Groups First Slice Seed Data
-- Source: ontology/zoela/connect-group-routes.ttl
-- Validates: autonomic matching, invite, refusal, OCEL/receipt discipline
--
-- Seed invariants:
--   person_with_consent → gets matched to group_a (capacity available, consent exists)
--   person_without_consent → gets REFUSED (A4: ConsentGate fails)
--   group_b → full (CapacityGate fails → waitlist)
--   private_group → requires human approval (A3: PolicyGate)

-- Campuses
INSERT INTO campuses (id, name, slug, location_mode) VALUES
  ('11111111-0000-0000-0000-000000000001', 'ZOE LA Main Campus', 'zoe-la-main', 'in-person'),
  ('11111111-0000-0000-0000-000000000002', 'ZOE LA Online', 'zoe-la-online', 'online')
ON CONFLICT (id) DO NOTHING;

-- Leaders and hosts
INSERT INTO persons (id, full_name, email, campus_id) VALUES
  ('22222222-0000-0000-0000-000000000001', 'Alex Leader', 'alex.leader@zoela.test', '11111111-0000-0000-0000-000000000001'),
  ('22222222-0000-0000-0000-000000000002', 'Jordan Host', 'jordan.host@zoela.test', '11111111-0000-0000-0000-000000000001')
ON CONFLICT (id) DO NOTHING;

-- Connect Groups
INSERT INTO connect_groups (id, name, campus_id, leader_id, host_id, max_capacity, current_count, is_private, location_mode, meeting_day, meeting_time) VALUES
  ('33333333-0000-0000-0000-000000000001', 'Group A — Sunday Evenings', '11111111-0000-0000-0000-000000000001', '22222222-0000-0000-0000-000000000001', '22222222-0000-0000-0000-000000000002', 12, 5, false, 'in-person', 'Sunday', '18:00'),
  ('33333333-0000-0000-0000-000000000002', 'Group B — Full Group', '11111111-0000-0000-0000-000000000001', '22222222-0000-0000-0000-000000000001', NULL, 10, 10, false, 'in-person', 'Wednesday', '19:00'),
  ('33333333-0000-0000-0000-000000000003', 'Private Leadership Circle', '11111111-0000-0000-0000-000000000001', '22222222-0000-0000-0000-000000000001', NULL, 8, 3, true, 'in-person', 'Friday', '19:00')
ON CONFLICT (id) DO NOTHING;

-- Persons seeking groups
INSERT INTO persons (id, full_name, email, campus_id) VALUES
  ('44444444-0000-0000-0000-000000000001', 'Sam WithConsent', 'sam.consent@zoela.test', '11111111-0000-0000-0000-000000000001'),
  ('44444444-0000-0000-0000-000000000002', 'Pat NoConsent', 'pat.noconsent@zoela.test', '11111111-0000-0000-0000-000000000001')
ON CONFLICT (id) DO NOTHING;

-- Consent records: Sam has consent, Pat does not
INSERT INTO consent_records (id, person_id, consent_type, granted, granted_at) VALUES
  ('55555555-0000-0000-0000-000000000001', '44444444-0000-0000-0000-000000000001', 'connect_group_matching', true, NOW()),
  ('55555555-0000-0000-0000-000000000002', '44444444-0000-0000-0000-000000000001', 'communication', true, NOW())
  -- Pat has NO consent records — ConsentGate will refuse
ON CONFLICT (id) DO NOTHING;

-- Group interests
-- Sam's interest → will be matched to Group A (capacity, consent OK → A1 autonomic)
INSERT INTO group_interests (id, person_id, campus_id, schedule_preference, location_preference, submitted_at) VALUES
  ('66666666-0000-0000-0000-000000000001', '44444444-0000-0000-0000-000000000001', '11111111-0000-0000-0000-000000000001', 'Sunday evenings', 'in-person', NOW()),
  -- Pat's interest → will be REFUSED (A4: ConsentGate fails)
  ('66666666-0000-0000-0000-000000000002', '44444444-0000-0000-0000-000000000002', '11111111-0000-0000-0000-000000000001', 'Sunday evenings', 'in-person', NOW())
ON CONFLICT (id) DO NOTHING;
