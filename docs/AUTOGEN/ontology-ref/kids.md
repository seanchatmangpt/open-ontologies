# kids.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/kids.ttl`
- **Triples:** 192
- **Classes:** 3 · **Properties:** 23 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `KidsCampRegistration` | Kids Camp Registration | A formal registration of a child for a ZOE Kids camp event, requiring guardian consent and medical consent. |
| `KidsCheckIn` | Kids Check-In | A guardian-verified check-in event recording a child's attendance at a ZOE Kids session. |
| `ZoeKidsSession` | ZOE Kids Session | A single scheduled ZOE Kids ministry session for a specific age group at a campus. |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `ageGroup` | ZoeKidsSession | Concept | The generation segment concept (ElementarySegment, PreschoolSegment, or InfantSe |
| `campEventId` | Thing | string | Foreign key referencing the camp event this registration is for. |
| `campRegistrationId` | KidsCampRegistration | string | Unique identifier for this kids camp registration record. |
| `campusId` | Thing | string | Identifier of the ZOE LA campus at which this resource is associated. |
| `checkedInAt` | KidsCheckIn | dateTime | ISO 8601 timestamp recording when the child was checked in to the session. |
| `checkedOutAt` | KidsCheckIn | dateTime | ISO 8601 timestamp recording when the child was checked out of the session. |
| `checkInId` | KidsCheckIn | string | Unique identifier for this specific check-in event. |
| `checkInMethod` | KidsCheckIn | string | Method used for check-in: 'qr' for QR code scan or 'manual' for staff-assisted e |
| `childId` | Thing | string | Foreign key referencing the child's person profile in the ZOE LA person registry |
| `guardianConsentId` | Thing | string | Foreign key referencing the guardian consent record authorizing participation. |
| `guardianConsentVerified` | KidsCheckIn | boolean | True when the guardian's consent was verified at the time of check-in. |
| `guardianId` | Thing | string | Foreign key referencing the guardian's person profile in the ZOE LA person regis |
| `leadTeacherId` | ZoeKidsSession | string | Profile identifier of the lead teacher or minister responsible for this kids ses |
| `maxCapacity` | ZoeKidsSession | integer | Maximum number of children permitted to attend this session. |
| `medicalConsentId` | Thing | string | Foreign key referencing the signed medical consent record required for camp part |
| `registeredAt` | Thing | dateTime | ISO 8601 timestamp recording when this registration was submitted. |
| `registrationStatus` | Thing | Concept | Current status of the registration as a SKOS concept from the relevant status sc |
| `requiresGuardianCheckIn` | ZoeKidsSession | boolean | Always true for ZOE Kids sessions; a guardian must physically check the child in |
| `roomLocation` | ZoeKidsSession | string | Physical room or space identifier where this kids session takes place. |
| `securityTag` | KidsCheckIn | string | A 4-digit alphanumeric security code printed on the child's and guardian's match |
| `sessionCode` | ZoeKidsSession | string | Unique alphanumeric code identifying this kids ministry session (e.g. KIDS-2026- |
| `sessionDate` | ZoeKidsSession | date | The calendar date on which this kids session is scheduled to occur. |
| `sessionId` | KidsCheckIn | string | Foreign key referencing the ZoeKidsSession this check-in is associated with. |
