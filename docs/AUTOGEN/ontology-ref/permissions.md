# permissions.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/permissions.ttl`
- **Triples:** 263
- **Classes:** 3 · **Properties:** 13 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `ZoePermission` | ZOE Permission | An ODRL Permission scoped to a ZOE LA data object type and sensitivity level. Grants a requiredRole (and above) the stat |
| `ZoePolicy` | ZOE Policy | An ODRL Policy that aggregates permissions and prohibitions for a named Supabase table. The rls_table property identifie |
| `ZoeProhibition` | ZOE Prohibition | An ODRL Prohibition explicitly blocking a blockedRole from performing an operation on a target object type. |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `appliesToMinistry` | ZoePolicy | Concept | Optionally scopes this policy to a specific ministry context concept. If absent, |
| `blockedRole` | ZoeProhibition | ZoeRole | The ZoeRole that is explicitly prohibited from performing the targeted operation |
| `hasSensitivityLevel` | ZoePolicy | Concept | The sensitivity concept that defines which rows this policy is intended to prote |
| `operation` | ZoePermission | string | The SQL/API operation this permission governs: one of read, write, delete, or as |
| `permissionCode` | ZoePermission | string | Short uppercase code uniquely identifying this permission rule; used as the RLS  |
| `policyCode` | ZoePolicy | string | Short uppercase code uniquely identifying this policy; used as the Supabase RLS  |
| `prohibitionCode` | ZoeProhibition | string | Short uppercase code uniquely identifying this prohibition rule. |
| `prohibitionTargetObjectType` | ZoeProhibition | string | The Supabase table name against which this prohibition applies. |
| `refusalReason` | ZoeProhibition | string | Human-readable explanation of why this prohibition exists; surfaced to the clien |
| `requiredRole` | ZoePermission | ZoeRole | The minimum ZoeRole (by permissionLevel) a user must hold to exercise this permi |
| `rls_table` | ZoePolicy | string | The Supabase/PostgreSQL table name for which this policy generates Row-Level Sec |
| `targetObjectType` | ZoePermission | string | The logical object class (Supabase table name) that this permission governs. |
| `targetSensitivity` | ZoePermission | Concept | Links this permission to the sensitivity concept that determines which rows are  |
