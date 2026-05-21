# routes.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/routes.ttl`
- **Triples:** 427
- **Classes:** 3 · **Properties:** 30 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `RouteInstance` | Route Instance | A live execution of a ServiceRoute for a specific subject (person, household, youth, etc.). Tracks current stage, start  |
| `RouteStage` | Route Stage | A named, ordered step within a ServiceRoute. Stages carry POWL partial-order edges via zoe:predecessorStage, enabling re |
| `ServiceRoute` | Service Route | The central organizing object for ministry service delivery. Every app action—need activation, volunteer assignment, con |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `completedAt` | RouteInstance | dateTime | ISO-8601 timestamp when the terminal stage of this route instance was completed. |
| `completionReceiptType` | RouteStage | string | Identifier for the ServiceReceipt subtype that is emitted when this stage is com |
| `currentStageCode` | RouteInstance | string | Stage code of the active RouteStage for this instance. Updated each time the rou |
| `emitsOcelEvents` | ServiceRoute | boolean | If true, route stage completions emit OCEL-2.0 event records for downstream proc |
| `hasReceipt` | RouteInstance | Thing | Links this route instance to a ServiceReceipt (defined in receipt.ttl) that prov |
| `hasRoute` | RouteStage | ServiceRoute | Links a RouteStage to the ServiceRoute it belongs to. Every stage must belong to |
| `instanceRouteId` | RouteInstance | string | UUID identifying this specific execution of a route. Used as the case ID in the  |
| `instanceSubjectId` | RouteInstance | string | Identifier of the person, household, or other entity that is the subject of this |
| `instanceSubjectType` | RouteInstance | string | The type of the route subject: 'person', 'household', 'youth', 'group', etc. Det |
| `isActive` | ServiceRoute | boolean | Whether this route definition is currently enabled for new instance creation. In |
| `isEntryStage` | RouteStage | boolean | True if this stage is the initial entry point for the route. An entry stage has  |
| `isTerminalStage` | RouteStage | boolean | True if this stage represents a route completion point. Completing a terminal st |
| `ocelEventType` | RouteStage | string | The OCEL-2.0 activity name emitted when this stage completes, e.g. 'food.deliver |
| `outcomeState` | RouteInstance | Concept | SKOS concept from zoe:OutcomeStateScheme representing the current or final state |
| `predecessorStage` | RouteStage | RouteStage | POWL partial-order edge declaring that this stage may only be entered after the  |
| `refusalReason` | RouteStage | string | Human-readable explanation emitted if this stage is a gate that refuses the tran |
| `requiredConsentType` | RouteStage | Concept | SKOS concept identifying the consent classification that the subject must have g |
| `requiredEvidenceType` | RouteStage | string | String identifier for the type of evidence that must be attached before this sta |
| `requiredRole` | RouteStage | Concept | SKOS concept identifying the organizational role (e.g. VolunteerRole, PastorRole |
| `requiresEvidence` | ServiceRoute | boolean | If true, at least one stage in this route demands physical or digital evidence ( |
| `routeCategory` | ServiceRoute | Concept | SKOS concept from zoe:RouteCategoryScheme classifying the type of service this r |
| `routeCode` | ServiceRoute | string | Machine-readable identifier key for this route definition, e.g. 'NEED_SUPPORT_V1 |
| `routeLabel` | ServiceRoute | string | Human-readable display name for this service route, shown in the ZOE LA Mobile U |
| `routeMinistry` | ServiceRoute | Organization | The ministry organizational unit responsible for executing this route. Links to  |
| `routeObjectType` | ServiceRoute | string | The type of subject this route acts upon: 'person', 'household', 'youth', 'famil |
| `stageCode` | RouteStage | string | Machine-readable identifier for this stage within its route, e.g. 'received', 'a |
| `stageLabel` | RouteStage | string | Human-readable display name for this route stage, shown in the ZOE LA Mobile UI  |
| `stageOrder` | RouteStage | integer | Integer position in the linear ordering of stages within the route. 0 for the en |
| `startedAt` | RouteInstance | dateTime | ISO-8601 timestamp when this route instance was created and the first stage ente |
| `timeoutHours` | RouteStage | decimal | Service Level Agreement (SLA) deadline for this stage in decimal hours. A route  |
