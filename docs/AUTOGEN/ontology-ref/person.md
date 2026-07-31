# person.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/person.ttl`
- **Triples:** 89
- **Classes:** 2 · **Properties:** 10 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `https://zoela.org/ontology/Household` | Household | A family or cohabiting group registered with ZOE LA, used to group PersonProfiles by physical address. |
| `https://zoela.org/ontology/PersonProfile` | Person Profile | A church member or visitor whose identity and household affiliation are tracked in the ZOE LA Mobile app. |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `https://zoela.org/ontology/addressLine1` | https://zoela.org/ontology/Household | string | First line of the household's street address. |
| `https://zoela.org/ontology/city` | https://zoela.org/ontology/Household | string | City in which the household is located. |
| `https://zoela.org/ontology/email` | https://zoela.org/ontology/PersonProfile | string | Primary email address used for login and notifications. |
| `https://zoela.org/ontology/firstName` | https://zoela.org/ontology/PersonProfile | string | Given (first) name of the person. |
| `https://zoela.org/ontology/householdId` | https://zoela.org/ontology/PersonProfile | https://zoela.org/ontology/Household | Links a PersonProfile to the Household to which the person belongs. |
| `https://zoela.org/ontology/householdName` | https://zoela.org/ontology/Household | string | Display name for the household, typically the primary family surname. |
| `https://zoela.org/ontology/lastName` | https://zoela.org/ontology/PersonProfile | string | Family (last) name of the person. |
| `https://zoela.org/ontology/phone` | https://zoela.org/ontology/PersonProfile | string | Primary phone number in E.164 format. |
| `https://zoela.org/ontology/role` | https://zoela.org/ontology/PersonProfile | string | Membership role of the person within ZOE LA (e.g., member, leader, visitor, staf |
| `https://zoela.org/ontology/zip` | https://zoela.org/ontology/Household | string | US postal (ZIP) code for the household's address. |
