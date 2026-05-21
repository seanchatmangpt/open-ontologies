# campus.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/campus.ttl`
- **Triples:** 104
- **Classes:** 2 · **Properties:** 8 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `Campus` | ZOE LA Campus | A physical church campus operated by ZOE LA. Represents both the administrative organization unit and the geographic pla |
| `CampusLocation` | Campus Location | The precise physical address, coordinates, and parking information for a ZOE LA campus. |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `campusCity` | Campus | string | City in which the campus is located (e.g. Los Angeles, Torrance). |
| `campusCode` | Campus | string | Short uppercase code uniquely identifying the campus (e.g. HLP, SBY, WAD). Used  |
| `campusTimezone` | Campus | string | IANA timezone string for the campus location (e.g. America/Los_Angeles). Used fo |
| `hasLocation` | Campus | CampusLocation | Links a campus to its detailed physical location record. |
| `isActive` | Campus | boolean | True when the campus is currently holding services; false when temporarily or pe |
| `latitude` | CampusLocation | decimal | WGS-84 latitude of the campus site in decimal degrees. |
| `longitude` | CampusLocation | decimal | WGS-84 longitude of the campus site in decimal degrees. |
| `parkingNotes` | CampusLocation | string | Human-readable guidance on parking availability and restrictions at the campus. |
