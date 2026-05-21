# ministry.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/ministry.ttl`
- **Triples:** 154
- **Classes:** 3 · **Properties:** 7 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `GenerationSegment` | Generation Segment | A SKOS concept representing a generational demographic cohort targeted by ministry activities. |
| `Ministry` | Ministry | An organisational ministry unit within ZOE LA Church. Each ministry has a defined mission focus, optional consent requir |
| `MinistryRole` | Ministry Role | A role held by a person within a ZOE LA ministry. Encodes permission level and receipt-creation authority for mobile app |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `canCreateReceipts` | MinistryRole | boolean | True when this role is authorised to emit manufacturing receipts on behalf of th |
| `hasCampus` | Ministry | Campus | Associates the ministry with the ZOE LA campus at which it operates. A ministry  |
| `ministryCode` | Ministry | string | Short uppercase code uniquely identifying the ministry (e.g. CARES, KIDS). Used  |
| `ministryFocus` | Ministry | string | Human-readable statement describing the primary mission and scope of the ministr |
| `requiresConsent` | Ministry | boolean | True when participation in this ministry requires explicit guardian or parental  |
| `rolePermissionLevel` | MinistryRole | integer | Integer permission level for this role (1 = read-only, 5 = admin). Controls whic |
| `targetGeneration` | Ministry | GenerationSegment | The generational demographic segment that this ministry primarily serves. |
