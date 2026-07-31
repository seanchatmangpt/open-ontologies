# connect-groups.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/connect-groups.ttl`
- **Triples:** 149
- **Classes:** 4 · **Properties:** 12 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `ConnectGroup` | Connect Group | A small-group community formation unit within ZOE LA. Functions as both a recurring gathering event and a formal organis |
| `GenerationSegmentTerm` | Generation Segment Term | A SKOS concept representing a generational age-band used to target ministry and Connect Group activities. |
| `GroupMembership` | Group Membership | The membership relationship linking a person to a Connect Group, recording their role, join date, and active status. |
| `YthConnectGroup` | Youth Connect Group | A Connect Group specifically designed for teenagers aged 12-17. Requires guardian consent for participation and applies  |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `currentSize` | ConnectGroup | integer | Current number of active members in the group. Compared against maxCapacity to c |
| `groupCode` | ConnectGroup | string | Short alphanumeric code uniquely identifying the Connect Group within its campus |
| `groupFrequency` | ConnectGroup | string | Recurrence cadence for group meetings. Allowed values: weekly, biweekly, monthly |
| `groupHasCampus` | ConnectGroup | Campus | Associates the Connect Group with the ZOE LA campus at which it primarily meets. |
| `groupLeaderId` | ConnectGroup | string | Identifier of the person holding the primary leadership role for this Connect Gr |
| `groupTargetGeneration` | ConnectGroup | GenerationSegmentTerm | The generation segment this Connect Group is designed to serve. Drawn from the G |
| `isOpen` | ConnectGroup | boolean | True when the group is accepting new members (currentSize < maxCapacity and grou |
| `joinedAt` | GroupMembership | dateTime | ISO-8601 datetime recording when the person joined the Connect Group. |
| `maxCapacity` | ConnectGroup | integer | Maximum number of members the group can accommodate. Used to determine open/clos |
| `memberIsActive` | GroupMembership | boolean | True when the membership is current and the person actively participates in the  |
| `memberRole` | GroupMembership | string | The role held by the member within this Connect Group (e.g. leader, co-leader, m |
| `requiresGuardianConsent` | YthConnectGroup | boolean | True (default) when participation by a minor in this youth group requires explic |
