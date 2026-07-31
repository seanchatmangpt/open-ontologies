# leadership-college.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/leadership-college.ttl`
- **Triples:** 202
- **Classes:** 4 · **Properties:** 24 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `LCCredential` | Leadership College Credential | A cryptographically signed credential issued to a person upon successful completion of a ZOE Leadership College cohort. |
| `LCEnrollment` | Leadership College Enrollment | A record of a person's enrollment in a ZOE Leadership College cohort, including completion tracking. |
| `LCModule` | Leadership College Module | A discrete curriculum module within a ZOE Leadership College cohort covering a specific leadership topic. |
| `LeadershipCollegeCohort` | Leadership College Cohort | A named cohort of students enrolled in a specific semester of ZOE Leadership College. |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `blake3Hash` | LCCredential | string | BLAKE3 cryptographic hash of this credential's canonical data, providing tamper- |
| `cohortCode` | LeadershipCollegeCohort | string | Unique code identifying this cohort (e.g. LC-2026-SPRING, LC-2026-FALL). |
| `cohortCodeRef` | LCCredential | string | The cohort code of the cohort for which this credential was issued (e.g. LC-2026 |
| `cohortId` | LCEnrollment | string | Foreign key referencing the LeadershipCollegeCohort this enrollment is for. |
| `cohortSeason` | LeadershipCollegeCohort | string | The academic season of this cohort: 'Spring' or 'Fall'. |
| `cohortYear` | LeadershipCollegeCohort | integer | The calendar year in which this Leadership College cohort runs. |
| `completedAt` | LCEnrollment | dateTime | ISO 8601 timestamp recording when this person completed the cohort program. |
| `completionReceiptId` | LCEnrollment | string | Foreign key referencing the receipt issued upon successful completion of the coh |
| `completionStatus` | LCEnrollment | Concept | Current completion status of this enrollment as a SKOS concept from LCCompletion |
| `credentialId` | LCCredential | string | Unique identifier for this Leadership College credential. |
| `credentialType` | LCCredential | string | Type of credential issued (e.g. COMPLETION-CERT, LEADERSHIP-CERT, MINISTRY-CERT) |
| `durationWeeks` | LCModule | integer | Number of weeks this module spans within the cohort schedule. |
| `endDate` | LeadershipCollegeCohort | date | The date on which this cohort's program concludes. |
| `enrolledAt` | LCEnrollment | dateTime | ISO 8601 timestamp recording when this person enrolled in the cohort. |
| `enrollmentId` | LCEnrollment | string | Unique identifier for this Leadership College enrollment record. |
| `hasCohort` | LCModule | LeadershipCollegeCohort | Links this curriculum module to the Leadership College cohort in which it is tau |
| `issuedAt` | LCCredential | dateTime | ISO 8601 timestamp recording when this credential was issued. |
| `leadFacilitatorId` | LeadershipCollegeCohort | string | Profile identifier of the lead facilitator responsible for this Leadership Colle |
| `maxEnrollment` | LeadershipCollegeCohort | integer | Maximum number of students permitted to enroll in this Leadership College cohort |
| `moduleCode` | LCModule | string | Unique code identifying this curriculum module (e.g. LC-MOD-01-IDENTITY). |
| `moduleSequence` | LCModule | integer | Ordinal position of this module within the cohort curriculum (1 = first module). |
| `moduleTitle` | LCModule | string | Human-readable title of this curriculum module (e.g. 'Identity in Christ'). |
| `personId` | Thing | string | Foreign key referencing the person's profile in the ZOE LA person registry. |
| `startDate` | LeadershipCollegeCohort | date | The date on which this cohort's program begins. |
