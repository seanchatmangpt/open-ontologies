# yth.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/yth.ttl`
- **Triples:** 169
- **Classes:** 3 · **Properties:** 18 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `YthCampRegistration` | YTH Camp Registration | A formal registration of a youth for a ZOE Youth camp event, requiring guardian and medical consent. |
| `YthConnectGroup` | YTH Connect Group | A recurring small-group meeting for ZOE Youth, organized by grade range and campus. |
| `YthNight` | YTH Night | A scheduled ZOE Youth ministry gathering event (weekly or special) for middle school or high school students. |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `eventDate` | YthNight | date | The calendar date on which this YTH Night event is scheduled. |
| `isActive` | YthConnectGroup | boolean | True when this connect group is currently active and accepting new members. |
| `leaderId` | YthConnectGroup | string | Profile identifier of the volunteer or staff leader facilitating this connect gr |
| `maxAttendees` | YthNight | integer | Maximum number of youth permitted to attend this YTH Night event. |
| `meetingDay` | YthConnectGroup | string | Day of the week on which this connect group meets (e.g. Monday, Wednesday, Frida |
| `meetingTime` | YthConnectGroup | string | Scheduled start time for this connect group in HH:MM format (24-hour). |
| `paymentStatus` | YthCampRegistration | string | Current payment status for this camp registration (e.g. PAID, PARTIAL, UNPAID). |
| `requiresGuardianConsent` | YthConnectGroup | boolean | Always true for ZOE Youth connect groups; guardian consent is required for youth |
| `requiresPreregistration` | YthNight | boolean | True when youth must pre-register to attend this YTH Night event. |
| `roomingPreference` | YthCampRegistration | string | Optional preference for cabin or room assignments (e.g. a preferred roommate nam |
| `scholarshipApplied` | YthCampRegistration | boolean | True when a financial assistance scholarship has been applied to this camp regis |
| `speakerName` | YthNight | string | Full name of the speaker or minister delivering the message at this YTH Night. |
| `targetGrade` | YthConnectGroup | string | Grade range served by this connect group: '6-8' for middle school or '9-12' for  |
| `targetSegment` | Thing | Concept | The generation segment concept (e.g. MiddleSchoolSegment, HighSchoolSegment) thi |
| `youthId` | YthCampRegistration | string | Foreign key referencing the youth's person profile in the ZOE LA person registry |
| `ythCampRegId` | YthCampRegistration | string | Unique identifier for this youth camp registration record. |
| `ythGroupCode` | YthConnectGroup | string | Unique code identifying this YTH Connect Group (e.g. YTH-CG-HS-EAST-MON). |
| `ythNightCode` | YthNight | string | Unique alphanumeric code identifying this YTH Night event (e.g. YTH-NIGHT-2026-0 |
