# ocel.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/ocel.ttl`
- **Triples:** 233
- **Classes:** 2 · **Properties:** 15 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `OcelEvent` | OCEL Event | An OCEL 2.0 event capturing a route-relevant action. These events form the raw event log consumed by wasm4pm for process |
| `OcelObjectRef` | OCEL Object Reference | An object reference in an OCEL 2.0 event. OCEL is object-centric, meaning each event can involve multiple typed objects  |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `hasEvent` | OcelObjectRef | OcelEvent | Links an OCEL object reference to the event in which it participates. |
| `ocelActorId` | OcelEvent | string | The user ID of the human or system actor who performed the route action. |
| `ocelCampusId` | OcelEvent | string | The identifier of the campus or physical location where this event occurred. |
| `ocelEventId` | OcelEvent | string | Unique UUID identifying this OCEL event instance. |
| `ocelEventType` | OcelEvent | Concept | The SKOS concept from zoe:OcelEventTypeScheme classifying what kind of route act |
| `ocelEvidenceId` | OcelEvent | string | Optional identifier of the evidence artifact that triggered or proves this event |
| `ocelMinistryCode` | OcelEvent | string | The code of the ministry team responsible for the route action captured by this  |
| `ocelObjectId` | OcelObjectRef | string | The identifier of the object involved in the OCEL event. |
| `ocelObjectType` | OcelObjectRef | string | The type category of the referenced object (e.g., person, household, need, resou |
| `ocelOutcomeState` | OcelEvent | string | The resulting state of the route or need object after this event was applied (e. |
| `ocelReceiptId` | OcelEvent | string | Optional identifier of the cryptographic receipt emitted alongside this event, i |
| `ocelRelation` | OcelObjectRef | string | Describes how this object relates to the event (e.g., subject, assignee, resourc |
| `ocelRouteId` | OcelEvent | string | The identifier of the care route instance this event belongs to. |
| `ocelRouteStageCode` | OcelEvent | string | The code identifying the specific route stage at which this event was emitted. |
| `ocelTimestamp` | OcelEvent | dateTime | The ISO-8601 date-time when the route action represented by this event occurred. |
