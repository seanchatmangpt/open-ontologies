# care.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/care.ttl`
- **Triples:** 274
- **Classes:** 5 · **Properties:** 25 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `CareRequest` | Care Request | A request for pastoral care, spiritual support, or a community service need submitted on behalf of an individual or hous |
| `CareRequestTypeTerm` | Care Request Type Term | An individual type term within the CareRequestType controlled vocabulary. |
| `FollowUpMethodTerm` | Follow-Up Method Term | An individual contact method term within the FollowUpMethod controlled vocabulary. |
| `FollowUpTask` | Follow-Up Task | A discrete, assignable pastoral action item created in response to a care request. Records the method, schedule, outcome |
| `PrayerRequest` | Prayer Request | A care request specifically for intercessory prayer. Extends CareRequest with public visibility, intercession count, and |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `assignedCareTeamMember` | CareRequest | string | Identifier of the pastoral care team member currently responsible for this reque |
| `assignedTo` | FollowUpTask | string | Identifier of the pastoral staff member or care team volunteer responsible for t |
| `careRequestId` | CareRequest | string | UUID primary key uniquely identifying this care request record. |
| `dueAt` | FollowUpTask | dateTime | ISO-8601 timestamp by which this follow-up task should be completed. |
| `escalated` | FollowUpTask | boolean | When true, this follow-up task has been escalated to pastoral leadership because |
| `followUpCareRequestId` | FollowUpTask | string | Identifier of the parent care request that generated this follow-up task. |
| `followUpDue` | CareRequest | dateTime | ISO-8601 timestamp of the earliest outstanding follow-up task due date for this  |
| `followUpId` | FollowUpTask | string | UUID primary key uniquely identifying this follow-up task. |
| `followUpMethod` | FollowUpTask | FollowUpMethodTerm | Controlled-vocabulary contact method for this follow-up task. |
| `followUpScheduled` | CareRequest | boolean | When true, at least one follow-up task has been scheduled for this care request. |
| `hasMedicalComponent` | CareRequest | boolean | When true, the request involves a medical situation. Triggers elevated privacy h |
| `householdId` | CareRequest | string | Identifier of the household associated with the care request subject. |
| `isAnonymous` | CareRequest | boolean | When true, the subject identity is withheld from general pastoral staff. Only de |
| `isComplete` | FollowUpTask | boolean | When true, this follow-up task has been completed and an outcome note has been r |
| `isPublic` | PrayerRequest | boolean | When true, this prayer request may be shared with the congregation prayer wall.  |
| `outcomeNote` | FollowUpTask | string | Narrative notes recorded by the assignee describing the outcome or contact made  |
| `prayedForCount` | PrayerRequest | integer | Number of members who have indicated they are interceding for this prayer reques |
| `prayerCampusId` | PrayerRequest | string | Identifier of the ZOE LA campus to which this prayer request is routed for pasto |
| `requestedAt` | CareRequest | dateTime | ISO-8601 timestamp when this care request was submitted. |
| `requestedBy` | CareRequest | string | Identifier of the person (member or staff) who submitted this care request. |
| `requestType` | CareRequest | CareRequestTypeTerm | Controlled-vocabulary type classifying the nature of this care request. |
| `requiresPastoral` | CareRequest | boolean | When true, this request must be reviewed and actioned by an ordained pastoral st |
| `sensitivityLevel` | CareRequest | Concept | SKOS concept from the policy sensitivity scheme indicating the privacy and acces |
| `subjectId` | CareRequest | string | Identifier of the individual who is the subject of this care request. May differ |
| `taskCompletedAt` | FollowUpTask | dateTime | ISO-8601 timestamp when this follow-up task was marked complete. |
