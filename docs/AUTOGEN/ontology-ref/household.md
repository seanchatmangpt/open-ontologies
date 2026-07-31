# household.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/household.ttl`
- **Triples:** 158
- **Classes:** 5 · **Properties:** 20 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `Household` | Household | A family or cohabiting group registered with ZOE LA, used as the primary unit for care requests, service routing, and ad |
| `HouseholdMember` | Household Member | Bridge record associating a PersonProfile with a Household, capturing the member's role within that household (e.g. head |
| `HouseholdNeed` | Household Need | Bridge record linking a Household to a Need, supporting a many-to-many relationship between households and service needs |
| `n4367bd1c47304354accf2f3486113011b1` | — | — |
| `n4367bd1c47304354accf2f3486113011b5` | — | — |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `addressLine1` | Household | string | Primary street address line for the household (e.g. '123 Main St'). |
| `addressLine2` | Household | string | Secondary address line for the household (e.g. apartment or unit number). |
| `campusId` | Household | string | Foreign key to the Campus the household is primarily affiliated with. Drives cam |
| `city` | Household | string | City component of the household's mailing address. |
| `createdAt` | Household | dateTime | ISO 8601 timestamp recording when the household record was first created in the  |
| `householdId` | Household, n4367bd1c47304354accf2f3486113011b1 | string | Unique surrogate identifier for the household record (UUID or database primary k |
| `householdMemberId` | HouseholdMember | string | Unique surrogate identifier for the household membership record. |
| `householdName` | Household | string | Display name for the household, typically the family surname (e.g. 'The Smith Fa |
| `householdNeedId` | HouseholdNeed | string | Unique surrogate identifier for the household-to-need bridge record. |
| `householdSize` | Household | integer | Count of individuals currently active in the household, used for resource sizing |
| `isActive` | Household, n4367bd1c47304354accf2f3486113011b5 | boolean | Boolean flag indicating whether the household is currently active and eligible t |
| `joinedAt` | HouseholdMember | dateTime | Timestamp when the person was added to the household membership record. |
| `membershipRole` | HouseholdMember | string | Role of the person within the household. Allowed values: head, spouse, child, ot |
| `needId` | HouseholdNeed | string | Foreign key to the Need that has been associated with this household. |
| `personId` | HouseholdMember | string | Foreign key to the PersonProfile that belongs to this household membership. |
| `primaryContactId` | Household | string | Foreign key to the PersonProfile designated as the primary point of contact for  |
| `reportedAt` | HouseholdNeed | dateTime | Timestamp when this household-to-need association was first recorded. |
| `sensitivityLevel` | Household | string | Data sensitivity classification for the household record. Allowed values: Public |
| `state` | Household | string | State or province component of the household's mailing address (ISO 3166-2 subdi |
| `zipCode` | Household | string | Postal/ZIP code for the household's address, used for campus assignment and geog |
