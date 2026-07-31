# partners.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/partners.ttl`
- **Triples:** 211
- **Classes:** 2 · **Properties:** 18 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `PartnerCapacity` | Partner Capacity | A snapshot of a partner organization's current service capacity and availability for a given resource type. |
| `PartnerOrganization` | Partner Organization | An organization that ZOE LA has a formal partnership with to provide services to individuals and families in need. |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `availabilityUpdatedAt` | PartnerCapacity | dateTime | The date and time when the current availability count was last updated. |
| `canReceiveReferrals` | PartnerOrganization | boolean | Indicates whether the partner organization is currently accepting referrals from |
| `capacityPartnerId` | PartnerCapacity | string | The identifier of the partner organization to which this capacity record belongs |
| `currentAvailability` | PartnerCapacity | integer | The number of service slots currently available at the partner organization for  |
| `isActive` | PartnerOrganization | boolean | Indicates whether the partnership with this organization is currently active. |
| `maxWeeklyReferrals` | PartnerOrganization | integer | The maximum number of referrals this partner organization can accept in a single |
| `partnerCategory` | PartnerOrganization | Concept | The service category of the partner organization, drawn from the zoe:PartnerCate |
| `partnerCode` | PartnerOrganization | string | A unique short code identifying the partner organization, e.g. 'LA-FOOD-BANK'. |
| `partnerContactEmail` | PartnerOrganization | string | The email address of the primary contact person at the partner organization. |
| `partnerContactName` | PartnerOrganization | string | The full name of the primary contact person at the partner organization. |
| `partnerContactPhone` | PartnerOrganization | string | The phone number of the primary contact person at the partner organization. |
| `partnerMouDate` | PartnerOrganization | date | The date on which the Memorandum of Understanding (MOU) between ZOE LA and the p |
| `partnerWebsite` | PartnerOrganization | string | The public website URL of the partner organization. |
| `requiresConsentForReferral` | PartnerOrganization | boolean | Indicates whether this partner organization requires explicit consent from the s |
| `resourceType` | PartnerCapacity | string | The type of resource or service this capacity record describes, e.g. food, cloth |
| `serviceAreaZips` | PartnerOrganization | string | A comma-separated list of ZIP codes representing the geographic service area of  |
| `waitlistCount` | PartnerCapacity | integer | The number of individuals or households currently on the waitlist for this resou |
| `weeklyCapacity` | PartnerCapacity | integer | The total weekly service capacity of the partner organization for this resource  |
