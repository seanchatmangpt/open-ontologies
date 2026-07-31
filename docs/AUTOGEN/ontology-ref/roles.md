# roles.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/roles.ttl`
- **Triples:** 217
- **Classes:** 2 · **Properties:** 13 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `PersonRoleAssignment` | Person Role Assignment | A time-bounded assignment of a ZoeRole to a person profile within a ministry context. Subclass of org:Membership. |
| `ZoeRole` | ZOE Role | An operational role within ZOE LA's ministry structure. Each role instance is a SKOS Concept in zoe:RoleScheme and carri |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `assignedAt` | PersonRoleAssignment | dateTime | ISO-8601 timestamp at which the role assignment was created. |
| `assignedBy` | PersonRoleAssignment | string | UUID of the AdminRole or PastorRole user who created this assignment. |
| `canAssignVolunteers` | ZoeRole | boolean | Indicates whether this role may assign volunteers to service routes and events. |
| `canCreateReceipts` | ZoeRole | boolean | Indicates whether this role may write service receipts to the receipts table. |
| `canViewSensitive` | ZoeRole | boolean | Indicates whether this role is permitted to read youth-protected and pastoral-pr |
| `expiresAt` | PersonRoleAssignment | dateTime | Optional ISO-8601 timestamp after which the assignment is no longer active. Null |
| `hasMinistry` | PersonRoleAssignment | Concept | Links a PersonRoleAssignment to the ministry context in which this role is exerc |
| `hasRole` | PersonRoleAssignment | ZoeRole | Links a PersonRoleAssignment to the ZoeRole concept it grants. |
| `isActive` | PersonRoleAssignment | boolean | Boolean flag. False when the assignment has been revoked or has expired. |
| `permissionLevel` | ZoeRole | integer | Ordinal privilege level (1 = least, 5 = most). RLS policies compare this integer |
| `requiresBackgroundCheck` | ZoeRole | boolean | When true, an approved background check (BGC consent) must be on file before thi |
| `requiresTraining` | ZoeRole | boolean | When true, the role cannot be assigned until the person has completed required t |
| `roleCode` | ZoeRole | string | Short alphanumeric code identifying the role; stored in the database profiles.ro |
