# evidence.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/evidence.ttl`
- **Triples:** 300
- **Classes:** 7 · **Properties:** 30 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `AssignmentEvidence` | Assignment Evidence | Evidence that a volunteer or physical resource was formally assigned to fulfill a service need at a route stage. |
| `AttendanceEvidence` | Attendance Evidence | Evidence that a participant checked in to an event or service location, recorded by QR scan, manual entry, or kiosk. |
| `ConsentEvidence` | Consent Evidence | Evidence that informed consent was obtained from a participant or their guardian before a sensitive route stage action. |
| `DeliveryEvidence` | Delivery Evidence | Evidence that a service or physical resource was delivered to the intended recipient, optionally including a recipient s |
| `Evidence` | Evidence | An evidence artifact that proves a route stage action occurred. Evidence is required before a route can advance to the n |
| `FollowUpEvidence` | Follow-Up Evidence | Evidence that a scheduled follow-up contact (call, text, or in-person visit) was completed with an outcome recorded. |
| `FormSubmissionEvidence` | Form Submission Evidence | Evidence that a structured digital or paper form was submitted, capturing intake, assessment, or registration data for a |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `assignedAt` | AssignmentEvidence | dateTime | The ISO-8601 date-time when the volunteer or resource assignment was confirmed. |
| `assignedResourceId` | AssignmentEvidence | string | The identifier of the physical resource (food bag, clothing bundle, etc.) assign |
| `assignedVolunteerId` | AssignmentEvidence | string | The user ID of the volunteer assigned to the route stage. |
| `blake3Hash` | Evidence | string | BLAKE3 cryptographic hash of the evidence payload, enabling tamper detection. |
| `capturedAt` | Evidence | dateTime | The ISO-8601 date-time when this evidence was captured. |
| `capturedBy` | Evidence | string | The user ID of the actor who captured this evidence. |
| `checkedInAt` | AttendanceEvidence | dateTime | The ISO-8601 date-time when the participant's check-in was recorded. |
| `checkInMethod` | AttendanceEvidence | string | The method used to record attendance: qr (QR code scan), manual (staff entry), o |
| `consentTypeCode` | ConsentEvidence | string | A code identifying the type of consent (e.g., data-sharing, minor-participation, |
| `deliveredAt` | DeliveryEvidence | dateTime | The ISO-8601 date-time when the delivery was completed. |
| `deliveredQuantity` | DeliveryEvidence | integer | The count of units (food bags, items, etc.) that were delivered to the recipient |
| `deliveryLocation` | DeliveryEvidence | string | A description or code identifying where the delivery took place (campus, address |
| `documentUrl` | Evidence | string | Optional URL of a stored document (e.g., signed form, photo) that constitutes or |
| `eventId` | AttendanceEvidence | string | The identifier of the event or service session the participant attended. |
| `evidenceId` | Evidence | string | Unique identifier (UUID) for this evidence artifact. |
| `evidenceType` | Evidence | Concept | The SKOS concept from zoe:EvidenceTypeScheme classifying the kind of evidence. |
| `followUpMethod` | FollowUpEvidence | string | The channel used for follow-up: call (phone call), text (SMS/messaging), or visi |
| `followUpOutcome` | FollowUpEvidence | string | A textual description or code capturing the result of the follow-up contact. |
| `formId` | FormSubmissionEvidence | string | The identifier of the form definition that was submitted. |
| `formVersion` | FormSubmissionEvidence | string | The version string of the form definition at the time of submission. |
| `guardianId` | ConsentEvidence | string | The user ID of the guardian who provided consent on behalf of a minor participan |
| `guardianRelationship` | ConsentEvidence | string | The relationship of the consenting guardian to the participant (e.g., parent, le |
| `isVerified` | Evidence | boolean | Boolean indicating whether this evidence has been independently verified by an a |
| `nextFollowUpDue` | FollowUpEvidence | dateTime | The ISO-8601 date-time when the next follow-up contact should occur, if one is r |
| `recipientSignature` | DeliveryEvidence | string | Base64-encoded signature image or cryptographic token from the recipient acknowl |
| `routeInstanceId` | Evidence | string | The identifier of the live care route instance this evidence belongs to. |
| `routeStageCode` | Evidence | string | The code identifying the route stage this evidence proves was completed. |
| `subjectId` | Evidence | string | The identifier of the person or household being served by this evidence's route  |
| `submissionPayloadHash` | FormSubmissionEvidence | string | BLAKE3 hash of the serialized form submission payload, used to detect tampering. |
| `verifiedBy` | Evidence | string | The user ID of the actor who verified this evidence. |
