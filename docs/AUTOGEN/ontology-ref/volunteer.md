# volunteer.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/volunteer.ttl`
- **Triples:** 193
- **Classes:** 4 · **Properties:** 21 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `AssignmentStatusTerm` | Assignment Status Term | An individual lifecycle state within the AssignmentStatus controlled vocabulary. |
| `ServeTeamAssignment` | Serve Team Assignment | A VolunteerAssignment scoped to a specific serve-team position at a dated event. Extends the base assignment with team a |
| `VolunteerAssignment` | Volunteer Assignment | A record of a specific volunteer being assigned to a specific opportunity. Carries provenance metadata and lifecycle sta |
| `VolunteerOpportunity` | Volunteer Opportunity | A defined role or position available for volunteers within a ministry or event at ZOE LA. |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `assignedAt` | VolunteerAssignment | dateTime | ISO-8601 timestamp when the assignment record was created. |
| `assignmentId` | VolunteerAssignment | string | UUID primary key uniquely identifying this volunteer assignment record. |
| `assignmentStatus` | VolunteerAssignment | AssignmentStatusTerm | Current lifecycle state of this assignment from the AssignmentStatus controlled  |
| `completedAt` | VolunteerAssignment | dateTime | ISO-8601 timestamp when the assignment was marked complete by the ministry lead  |
| `confirmedAt` | VolunteerAssignment | dateTime | ISO-8601 timestamp when the volunteer confirmed their acceptance of this assignm |
| `eventDate` | ServeTeamAssignment | date | Calendar date of the event for which this serve-team assignment is scheduled. |
| `eventId` | VolunteerOpportunity | string | Optional identifier linking this opportunity to a specific scheduled event. Abse |
| `hoursLogged` | VolunteerAssignment | decimal | Total number of service hours logged for this assignment upon completion. |
| `isActive` | VolunteerOpportunity | boolean | When true, this opportunity is open for new volunteer assignments. When false, i |
| `maxVolunteers` | VolunteerOpportunity | integer | Maximum number of volunteers that may be simultaneously assigned to this opportu |
| `minimumHoursPerMonth` | VolunteerOpportunity | decimal | Minimum number of service hours per calendar month expected for this opportunity |
| `ministryCode` | VolunteerOpportunity | string | Code of the ministry context in which this volunteer opportunity operates. |
| `opportunityCode` | VolunteerOpportunity | string | Short unique code identifying this volunteer opportunity (e.g. 'FOOD-SERVE-01'). |
| `opportunityDescription` | VolunteerOpportunity | string | Human-readable narrative describing the duties, schedule, and expectations of th |
| `opportunityId` | VolunteerAssignment | string | Identifier of the volunteer opportunity to which this person has been assigned. |
| `receiptId` | VolunteerAssignment | string | Identifier of the proof receipt emitted when this assignment reached a terminal  |
| `requiresConsent` | VolunteerOpportunity | boolean | When true, the volunteer must have a current signed consent on record before bei |
| `requiresRole` | VolunteerOpportunity | Concept | SKOS concept denoting the ministry role required before a volunteer may be assig |
| `routeInstanceId` | VolunteerAssignment | string | Identifier of the service route instance that generated or owns this assignment. |
| `serveTeamCode` | ServeTeamAssignment | string | Code identifying the serve team to which this assignment belongs (e.g. 'WORSHIP- |
| `volunteerId` | VolunteerAssignment | string | Identifier of the person assigned to serve in this opportunity. |
