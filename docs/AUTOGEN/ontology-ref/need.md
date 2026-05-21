# need.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/need.ttl`
- **Triples:** 211
- **Classes:** 4 · **Properties:** 15 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `FollowUpTask` | Follow-Up Task | A discrete action item created in response to a fulfilled or escalated need. Tracks post-delivery care continuity. |
| `Need` | Need | A request for care, support, or service submitted by or on behalf of a community member. Routed to volunteers or ministr |
| `NeedCategoryTerm` | Need Category Term | An individual term within the NeedCategory controlled vocabulary. |
| `NeedStatusTerm` | Need Status Term | An individual lifecycle state within the NeedStatus controlled vocabulary. |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `assignedTo` | Need | string | Identifier of the volunteer or ministry lead currently assigned to fulfill this  |
| `completedAt` | FollowUpTask | dateTime | ISO-8601 timestamp when the follow-up task was marked complete. |
| `createdAt` | Need | dateTime | ISO-8601 timestamp when the need was submitted. |
| `dueAt` | Need | dateTime | Optional deadline by which the need should be fulfilled. |
| `followUpAssignedTo` | FollowUpTask | string | Identifier of the volunteer or staff member responsible for completing this foll |
| `isPrivate` | Need | boolean | When true, only pastoral-level staff may view this need. Hides from general volu |
| `needCategory` | Need | NeedCategoryTerm | Controlled-vocabulary category classifying the type of care or service requested |
| `needDescription` | Need | string | Detailed narrative describing the nature and context of the need. |
| `needTitle` | Need | string | Short human-readable summary of the need (e.g. 'Groceries for family of four'). |
| `relatedNeedId` | FollowUpTask | string | Identifier of the parent Need that generated this follow-up task. |
| `requesterId` | Need | string | Identifier of the member or pastoral contact who submitted this need. |
| `status` | Need | NeedStatusTerm | Current lifecycle state of the need (Open, Assigned, InProgress, Fulfilled, Esca |
| `taskDescription` | FollowUpTask | string | Narrative description of the follow-up action to be taken. |
| `taskDueAt` | FollowUpTask | dateTime | Optional deadline by which the follow-up task should be completed. |
| `urgency` | Need | integer | Integer priority score for the need (1 = routine, 5 = critical). Used to sort ro |
