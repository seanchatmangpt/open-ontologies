# connect-group-routes.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/connect-group-routes.ttl`
- **Triples:** 385
- **Classes:** 5 · **Properties:** 30 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `AdmissionGate` | Admission Gate | A named gate condition that must evaluate to true before a route stage may execute. Gates enforce consent, capacity, sch |
| `GroupAttendance` | Group Attendance | An attendance record for a person at a Connect Group meeting. Required as evidence for stage 6 (First Meeting Attended). |
| `GroupInterest` | Group Interest | An expression of interest by a person seeking to join a Connect Group. Captures schedule/location preferences and drives |
| `GroupInvite` | Group Invite | An invite issued to a person to join a specific Connect Group after an autonomic or manual match. Gated by Communication |
| `GroupMembership` | Group Membership | Active membership of a person in a Connect Group. Terminal artifact of the ConnectGroupJoinRoute. Carries start date, st |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `attendanceByPerson` | GroupAttendance | PersonProfile | The PersonProfile whose attendance is being recorded. |
| `attendanceForGroup` | GroupAttendance | ConnectGroup | Links an attendance record to the ConnectGroup meeting at which attendance is re |
| `attendanceIsFirst` | GroupAttendance | boolean | True when this is the person's first attendance at any meeting of this Connect G |
| `attendanceMeetingDate` | GroupAttendance | date | Calendar date of the Connect Group meeting for which attendance is recorded. |
| `attendanceStatus` | GroupAttendance | string | Status of this attendance record. Allowed values: attended, absent, excused. |
| `autonomicActionClass` | RouteStage | Concept | A0-A4 risk classification (from AutonomicActionClassScheme) for the autonomic ac |
| `gateCode` | AdmissionGate | string | Machine-readable identifier for an AdmissionGate, used in runtime gate evaluatio |
| `interestForGroup` | GroupInterest | ConnectGroup | Links a GroupInterest to the specific ConnectGroup the person is interested in,  |
| `interestFromPerson` | GroupInterest | PersonProfile | Links a GroupInterest to the PersonProfile who submitted it. |
| `interestLocationPreference` | GroupInterest | string | Preferred geographic area or campus for Connect Group placement (e.g. 'Hollywood |
| `interestNotes` | GroupInterest | string | Optional free-text notes from the person expressing interest, e.g. life stage, m |
| `interestSchedulePreference` | GroupInterest | string | Free-text or structured schedule preference (e.g. 'Tuesday evenings', 'Sunday mo |
| `interestStatus` | GroupInterest | string | Lifecycle status of the GroupInterest. Allowed values: pending, matched, waitlis |
| `interestSubmittedAt` | GroupInterest | dateTime | ISO-8601 timestamp when the GroupInterest was submitted. Used as the OCEL event  |
| `inviteAcceptedAt` | GroupInvite | dateTime | ISO-8601 timestamp when the person accepted the invite. Null if not yet accepted |
| `inviteDeclinedAt` | GroupInvite | dateTime | ISO-8601 timestamp when the person declined the invite. Null if not declined. |
| `inviteExpiresAt` | GroupInvite | dateTime | ISO-8601 timestamp after which an unaccepted invite is automatically expired and |
| `inviteForGroup` | GroupInvite | ConnectGroup | The ConnectGroup to which the person is being invited. |
| `inviteSentAt` | GroupInvite | dateTime | ISO-8601 timestamp when the invite was dispatched. Maps to stage 4 OCEL event ti |
| `inviteStatus` | GroupInvite | string | Lifecycle status of the GroupInvite. Allowed values: sent, accepted, declined, e |
| `inviteToPerson` | GroupInvite | PersonProfile | The PersonProfile receiving this invite. |
| `isAutonomicRoute` | ServiceRoute | boolean | True when this route is designed to advance through stages autonomically without |
| `membershipByPerson` | GroupMembership | PersonProfile | The PersonProfile who holds this group membership. |
| `membershipForGroup` | GroupMembership | ConnectGroup | Links a GroupMembership record to the ConnectGroup the person is a member of. |
| `membershipStartDate` | GroupMembership | date | Calendar date on which active membership began. Set when stage 8 (Membership Act |
| `membershipStatus` | GroupMembership | string | Lifecycle status of the membership record. Allowed values: active, inactive, wai |
| `requiresGate` | RouteStage | AdmissionGate | An AdmissionGate that must pass before this stage may execute. A stage may requi |
| `routeStageForRoute` | RouteStage | ServiceRoute | Links a RouteStage to the ServiceRoute that contains it. Complementary to zoe:ha |
| `routeVersion` | ServiceRoute | string | Semantic version string for the route declaration (e.g. '1.0'). Updated when the |
| `stageOcelEventType` | RouteStage | string | The OCEL-2.0 activity identifier emitted when this stage completes. Used by wasm |
