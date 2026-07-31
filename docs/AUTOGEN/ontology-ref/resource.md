# resource.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/resource.ttl`
- **Triples:** 222
- **Classes:** 4 · **Properties:** 24 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `InventoryBatch` | Inventory Batch | A specific received lot of a resource type at a campus. Tracks quantities received, available, and distributed with prov |
| `Resource` | Resource | A tangible or intangible resource managed by ZOE LA for distribution to community members. Resources may be physical goo |
| `ResourceCategoryTerm` | Resource Category Term | An individual category term within the ResourceCategory controlled vocabulary. |
| `ResourceNeedMatch` | Resource Need Match | A bridge object recording a lawful many-to-many relationship between a service need and a matching resource allocation.  |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `batchId` | InventoryBatch | string | UUID primary key uniquely identifying this inventory batch. |
| `batchResourceCode` | InventoryBatch | string | Code of the resource type contained in this batch. Foreign key to zoe:Resource. |
| `campusId` | InventoryBatch | string | Identifier of the ZOE LA campus where this inventory batch is stored. |
| `donorId` | InventoryBatch | string | Identifier of the individual or partner donor who contributed this batch. |
| `expiresAt` | InventoryBatch | dateTime | ISO-8601 timestamp when this batch expires. Relevant only when zoe:isPerishable  |
| `isPerishable` | Resource | boolean | When true, inventory batches of this resource have a meaningful expiry date that |
| `matchedAt` | ResourceNeedMatch | dateTime | ISO-8601 timestamp when this need-resource match was created. |
| `matchId` | ResourceNeedMatch | string | UUID primary key uniquely identifying this need-resource match record. |
| `matchResourceCode` | ResourceNeedMatch | string | Code of the resource type allocated to satisfy this need. Foreign key to zoe:Res |
| `matchStatus` | ResourceNeedMatch | Concept | Current lifecycle state of this match from a SKOS controlled vocabulary. |
| `minimumAge` | Resource | integer | Minimum recipient age in years required to receive this resource (e.g. 18 for fi |
| `needId` | ResourceNeedMatch | string | Identifier of the service need being matched. Foreign key to the need domain. |
| `partnerRelayAllowed` | ResourceNeedMatch | boolean | When true, fulfillment of this match may be relayed to an external partner organ |
| `quantityAvailable` | InventoryBatch | decimal | Current quantity remaining in this batch available for distribution. |
| `quantityDistributed` | InventoryBatch | decimal | Cumulative quantity of this batch that has been distributed to community members |
| `quantityMatched` | ResourceNeedMatch | decimal | Quantity of the resource (in resource units) allocated to this need match. |
| `quantityReceived` | InventoryBatch | decimal | Total quantity (in resource units) received when this batch was logged. |
| `receivedAt` | InventoryBatch | dateTime | ISO-8601 timestamp when this inventory batch was received and logged. |
| `resourceCategory` | Resource | ResourceCategoryTerm | Controlled-vocabulary category classifying this resource type for routing and re |
| `resourceCode` | Resource | string | Short unique code identifying this resource type across all campuses (e.g. 'FOOD |
| `resourceRequiresConsent` | Resource | boolean | When true, the recipient must have a current signed consent record before this r |
| `substituteAllowed` | ResourceNeedMatch | boolean | When true, the routing engine may fulfill this match with a resource from an equ |
| `supplierPartner` | Resource | string | Name or identifier of the external partner or donor organization that supplies t |
| `unit` | Resource | string | Unit of measure for this resource (lbs, items, hours, dollars). Used for quantit |
