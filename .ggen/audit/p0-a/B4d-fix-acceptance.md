# P0-B.4d-fix through P0-B.9 — Full-Stack Manufactured Acceptance

Date: 2026-05-19
Binary: `~/ggen/target/release/ggen` (v26.5.18)

## Doctrine enforced

> Tera renders facts. Tera does not repair facts. Tera does not choose
> authority. Tera does not hide ambiguity. Admission belongs upstream.

> Agents invoke ggen, never tera directly. Tera is internal to ggen.

> Generated rule output paths must be unique per row, or the rule must
> aggregate into a single bundled file. Per-row writes to a single
> path silently lose all but the last result.

## What's manufactured end-to-end (this session, total)

Source-of-truth TTL → ggen rule → signed receipt → working artifacts:

| Layer | Artifact | Manufactured |
|---|---|---|
| **Database schema** | 4 admitted CREATE TABLE blocks (push_notification, persons, households, connect_groups) | ✓ |
| **Database schema** | 17 ALTER TABLE ADD COLUMN body fields across the 4 tables | ✓ |
| **Database schema** | 2 governance reference tables (roles, person_role_assignments) + 4 role seeds | ✓ |
| **Database RLS** | 22 CREATE POLICY rows from ODRL permission tuples (USING/WITH CHECK branched by op) | ✓ |
| **Expo project** | apps/mobile/package.json + version-pinned deps | ✓ |
| **Expo project** | apps/mobile/app.json with Supabase config | ✓ |
| **Expo project** | apps/mobile/tsconfig.json with @zoela/* path aliases | ✓ |
| **Expo project** | apps/mobile/babel.config.js, metro.config.js | ✓ |
| **Expo project** | apps/mobile/App.tsx (RLS probe + entry screen mount) | ✓ |
| **Expo project** | apps/mobile/src/lib/supabaseClient.ts (env-overridable defaults) | ✓ |
| **App surface** | packages/screens/zoela.tsx — 14 exported screens | ✓ |
| **App surface** | packages/screens/adminScreens.tsx — 3 admin list screens | ✓ |
| **App surface** | packages/screens/adminDetailScreen.tsx — 3 admin detail screens | ✓ |
| **App surface** | packages/navigation/zoela.tsx + navigationStack.tsx — 7 tab navigators each | ✓ |
| **App surface** | packages/types/pushCards.ts — 9 push card configurations | ✓ |
| **App surface** | 24 more TS/TSX files (types, schemas, forms, routes, evidence) | ✓ |
| **Supabase edge** | supabase/functions/route-gate/index.ts | ✓ |

**36 unique manufactured files per `ggen sync` run**, all covered by an
Ed25519-signed receipt at `.ggen/receipts/latest.json`.

## What's still hand-written

| File | Why it stays hand-written | Path |
|---|---|---|
| `20260518000001_zoela_tables.sql` | Covers ~10 tables (persons, campuses, ministries, etc.) the admission census has only partially reached. Coexists peacefully with ggen via `CREATE TABLE IF NOT EXISTS` and `DROP TRIGGER IF EXISTS`. | supabase/migrations/ |
| `20260518000002_zoela_bridge_tables.sql` | group_memberships, group_invites — junction tables not yet in admission scope | supabase/migrations/ |
| `20260518000003_zoela_rls.sql` | Consent-aware RLS for connect_groups/consent_records | supabase/migrations/ |
| `20260518000004_zoela_ocel_events.sql` | OCEL substrate — admission for `ggen:OcelObject/EventMaterialization` not yet wired | supabase/migrations/ |
| `apps/mobile/scripts/verify-supabase.mjs` | Operations tool (CI host-side check), not app code | apps/mobile/scripts/ |
| `apps/mobile/README.md` | Human-facing doc | apps/mobile/ |
| `supabase/functions/autonomic-executor/index.ts` etc. | Edge functions with deep autonomic logic; only `route-gate` is currently manufactured | supabase/functions/ |
| `supabase/seeds/connect_groups.sql` | Test data seed; not source-law | supabase/seeds/ |

Removing the 4 hand-written migrations requires the admission census to
grow to cover every class they define. That's iterative ontology work:
add `skos:notation` + ODRL coverage per class, disambiguate
multi-anchor classes via priority.

## ggen receipt (final)

```
operation_id: 586979ce-1d40-4778-9736-cbc6eab8a59b
timestamp:    2026-05-19T05:12:08.053327Z
duration:     ~4.5 s (15 imports)
inputs:       15 imports (9 zoela base + 1 policy + 5 profiles)
outputs:      36 files with BLAKE3 hashes
signature:    Ed25519, 128 chars (PRESENT)
ggen version: 26.5.18
```

## End-to-end verification

```
$ supabase db reset --local
... Applying migration 20260518000001_zoela_tables.sql ... ✓
... (all 8 migrations apply cleanly) ...
Finished supabase db reset on branch ontostar-integration.

$ cd apps/mobile && node scripts/verify-supabase.mjs
✓ REST endpoint reachable
    HTTP 200 from http://127.0.0.1:54321/rest/v1/
✓ push_notification: anon SELECT denied by RLS
    rows: 0
✓ roles: seed data accessible
    rows: 4 (admin,member,ministry_lead,volunteer)
All checks passed.

$ pnpm typecheck
(silent — clean)
```

## Census (15-import scope)

| | |
|---|---|
| Imports loaded | 15 (9 zoela base + 1 policy + 5 profiles) |
| Admitted domain materializations | 4 (push_notification, persons, households, connect_groups) |
| Refusals | 0 |
| Governance tables | 2 (roles, person_role_assignments) |
| CREATE POLICY rows | 22 |
| Body columns ALTERed | 17 |
| Role seed rows | 4 |
| Manufactured Expo scaffold files | 7 (apps/mobile/*) |
| Manufactured packages files | 18 (packages/*) |
| Total ggen-emitted files per sync | 36 |

## Doctrine wins this session

| Defect class | Before | After |
|---|---|---|
| `FOR INSERT ... USING(...)` invalid SQL | 3 broken policies | 0 (template branches by op) |
| `assigned_to` referenced but never admitted as a column | 2 broken constraints | 0 (profile points at `sent_to_person`) |
| `person_role_assignments` / `roles` referenced but never generated | RLS failed at evaluation | Governance ontology + ggen rule emits them |
| Generated screens collapse (per-row writes to one file) | 13/14 screens lost | All survive (bundled rule pattern) |
| apps/mobile scaffold hand-written | 7 boilerplate files | All ggen-emitted from Expo manifest TTL |
| FK ordering in governance | `person_role_assignments` before `roles` | Explicit `ggen:governanceTableOrder` resolves |
| Trigger collision with hand-written migrations | `set_persons_updated_at already exists` | Templates emit `DROP TRIGGER IF EXISTS` first (idempotent) |

## Architecture (post-fix)

```
ontology/zoela/*.ttl              (church-service domain — public-anchor specializations)
ontology/zoela/policy.ttl         (ODRL policies + permissions/prohibitions)
ontology/profiles/zoela-supabase-materialization.ttl  (kinds + priorities)
ontology/profiles/public-role-shapes.ttl              (SHACL NodeShapes per kind)
ontology/profiles/zoela-supabase-rls.ttl              (ODRL action → RLS translation)
ontology/profiles/zoela-governance.ttl                (governance reference tables + seeds)
ontology/profiles/zoela-expo-app.ttl                  (Expo manifest: deps, paths, screens)
            │
            ▼
     ggen sync --audit   ──>   .ggen/receipts/latest.json (Ed25519-signed)
            │
            ├── extract-public-materializations.rq          ── admitted (kind, class, table)
            │                  ▼
            │     supabase-migration.tera                   ── 20260519000002_zoela_admitted_tables.sql
            │
            ├── extract-public-materialization-columns.rq   ── per-(class, prop)
            │                  ▼
            │     supabase-columns.tera                     ── 20260519000003_zoela_admitted_columns.sql
            │
            ├── extract-public-materialization-refusals.rq  ── refused candidates
            │                  ▼
            │     supabase-materialization-refusals.tera    ── .ggen/audit/p0-a/zoela-refusals.sql
            │
            ├── extract-governance-tables.rq                ── roles + assignments (FK-ordered)
            │                  ▼
            │     supabase-governance.tera                  ── 20260519000001_zoela_governance.sql
            │                                                  (includes seed INSERTs)
            │
            ├── extract-odrl-policies.rq                    ── ODRL × admitted table
            │                  ▼
            │     supabase-rls.tera                         ── 20260519000004_zoela_admitted_rls.sql
            │                                                  (branched USING / WITH CHECK)
            │
            ├── extract-expo-package.rq                     ── package.json
            ├── extract-expo-app-json.rq                    ── app.json
            ├── extract-expo-tsconfig.rq                    ── tsconfig.json
            ├── extract-expo-runtime-config.rq              ── App.tsx + supabaseClient.ts
            └── extract-expo-manifest-id.rq                 ── babel/metro static configs

… and 22 other ggen rules producing TypeScript types, Zod schemas,
   React Native screens, navigation, OCEL events, etc.
```

## Known limitations (P1 follow-ups)

1. **Hand-written migrations remain** (20260518000001–4) — they cover ~10 tables not yet in the admission census. Coexistence works via `CREATE TABLE IF NOT EXISTS` and `DROP TRIGGER IF EXISTS`, but the schema is duplicated where ggen and hand-written overlap (e.g., `persons` has both `full_name` from hand-written and `first_name`/`last_name` from ggen). Resolve by expanding admission to cover all hand-written tables, then deleting the hand-written migrations.
2. **ggen pre-flight cliff** — 41 imports times out at >5min under v26.5.18. Current working cap is ~15 imports. This caps how many domain TTLs can be loaded simultaneously, which caps the admitted census.
3. **RLS policy name truncation** — 5 policy names exceed Postgres's 63-char identifier limit. Names are silently truncated; policies still create. Fix: shorter naming scheme (hash suffix instead of full action name).
4. **Expo CLI requires Node 20 LTS** — Expo SDK 52's `expo-modules-core` declares `main: src/index.ts` which Node 22.13+ can't load. The Supabase Docker stack, migrations, `verify-supabase.mjs`, and `pnpm typecheck` work on any Node.
5. **No real auth flow yet** — the Expo shell uses anon access only. Login + `person_role_assignments` rows + authenticated read paths are next.
6. **`~/.local/bin/ggen`** is still v26.5.5; only `~/ggen/target/release/ggen` is v26.5.18. PATH-resolved `ggen` is stale.
7. **OcelObjectMaterialization / OcelEventMaterialization** are wired but no ZOE class has the proper anchor + notation yet, so the OCEL substrate stays hand-written.

## Memory entries (durable principles)

- `feedback_render_not_repair.md` — Tera renders facts; never adjudicates.
- `feedback_ggen_is_manufacturing_authority.md` — Agents invoke ggen, never tera.
- `feedback_public_vocab_materialization.md` — Profile layer = source-law.
- `feedback_governance_no_rls.md` — Governance tables can't have RLS (chicken-and-egg).
- `project_projection_role_taxonomy.md` — Deferred: decompose `ggen:emitsSql`.

## Manufactured rules count

- 1 inference rule (normalize-zoela-classes)
- 36 generation rules across 4 categories:
  - **Schema** (4): admitted_tables, admitted_columns, admitted_rls, governance
  - **Audit** (1): refusals
  - **Expo scaffold** (7): package.json, app.json, tsconfig, babel.config, metro.config, App.tsx, supabaseClient.ts
  - **App surface** (24): types, schemas, screens, navigation, forms, routes, evidence, edge functions
