# connect-group-capacity.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/connect-group-capacity.ttl`
- **Triples:** 202
- **Classes:** 3 · **Properties:** 16 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `GroupCapacityRule` | Group Capacity Rule | Capacity configuration for a Connect Group. Declares the maximum number of members the group can hold, the minimum viabl |
| `SpotReservation` | Spot Reservation | A temporary spot hold created during the connect-group invite flow. A reservation keeps one spot off the open-capacity c |
| `WaitlistEntry` | Waitlist Entry | An ordered entry on the Connect Group waitlist created when a person expresses interest in a group that is at full capac |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `allowsWaitlist` | GroupCapacityRule | boolean | True when the group accepts waitlist entries after reaching maxCapacity. If fals |
| `capacityForGroup` | GroupCapacityRule | ConnectGroup | Associates the GroupCapacityRule with the Connect Group it governs. Each Connect |
| `capacityState` | GroupCapacityRule | Concept | Current capacity state concept from zoe:CapacityStateScheme: OpenCapacity, NearC |
| `maxCapacityRule` | GroupCapacityRule | integer | Maximum total number of active members the group can accommodate. When currentSi |
| `maxWaitlistSize` | GroupCapacityRule | integer | Maximum number of WaitlistEntry records permitted on this group's waitlist. Once |
| `minCapacity` | GroupCapacityRule | integer | Minimum viable number of members for the group to remain active. If attendance d |
| `reservationExpiresAt` | SpotReservation | dateTime | ISO-8601 datetime after which this reservation automatically transitions to 'exp |
| `reservationForGroup` | SpotReservation | ConnectGroup | Associates the SpotReservation with the Connect Group for which the spot is bein |
| `reservationForPerson` | SpotReservation | PersonProfile | Links the SpotReservation to the PersonProfile for whom the spot is being held d |
| `reservationStatus` | SpotReservation | string | Current lifecycle status of the reservation. Allowed values: 'held' (spot locked |
| `reservedSpots` | GroupCapacityRule | integer | Number of capacity slots held in reserve. Reserved spots do not count towards th |
| `waitlistCreatedAt` | WaitlistEntry | dateTime | ISO-8601 datetime when the person joined the waitlist. Used as the tiebreaker wh |
| `waitlistForGroup` | WaitlistEntry | ConnectGroup | Associates the WaitlistEntry with the Connect Group the person is waiting to joi |
| `waitlistPerson` | WaitlistEntry | PersonProfile | Links the WaitlistEntry to the PersonProfile who is waiting for a spot in the gr |
| `waitlistPosition` | WaitlistEntry | integer | 1-based integer position of this entry in the group's waitlist. Position 1 is ne |
| `waitlistStatus` | WaitlistEntry | string | Current status of the waitlist entry. Allowed values: 'waiting' (in queue), 'off |
