# core.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/core.ttl`
- **Triples:** 218
- **Classes:** 21 · **Properties:** 16 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `AppScreen` | App Screen | A mobile app screen or navigation destination in the ZOE LA app — from navigation.ttl |
| `CategoryRegistry` | Category Registry | A registry of ministry and service categories used for navigation and filtering — from categories.ttl |
| `EventRegistration` | Event Registration | A person's registration record for a specific ZOE Event (canonical IRI). |
| `Household` | Household | A household grouping one or more PersonProfiles at a shared address (canonical IRI). |
| `HouseholdMember` | Household Member | A person's membership role within a household grouping — from household.ttl |
| `HouseholdNeed` | Household Need | A care or resource need attributed to an entire household rather than a single person — from household.ttl |
| `KidsCampRegistration` | Kids Camp Registration | A child's registration for a ZOE Kids camp event — from kids.ttl |
| `KidsCheckIn` | Kids Check-In | A check-in record for a child at a ZOE Kids session — from kids.ttl |
| `LCCredential` | LC Credential | A credential awarded upon completing Leadership College requirements — from leadership-college.ttl |
| `LCEnrollment` | LC Enrollment | A person's enrollment record for a Leadership College module — from leadership-college.ttl |
| `LCModule` | LC Module | A single curriculum module within a Leadership College cohort — from leadership-college.ttl |
| `LeadershipCollegeCohort` | Leadership College Cohort | A ZOE Leadership College cohort group — from leadership-college.ttl |
| `PartnerOrganization` | Partner Organization | An external organization (social service agency, church partner, etc.) to which ZOE LA may refer members. Forward-declar |
| `PersonProfile` | Person Profile | A registered person in the ZOE LA Mobile system (canonical IRI). |
| `Referral` | Referral | A directed referral of a member to a PartnerOrganization or internal ministry. Forward-declared pending referrals.ttl. |
| `RouteOutcome` | Route Outcome | The resolved outcome of a ServiceRoute instance (completed, withdrawn, referred, etc.). Forward-declared pending outcome |
| `YthCampRegistration` | Yth Camp Registration | A youth participant's registration for a ZOE Youth camp — from yth.ttl |
| `YthConnectGroup` | Yth Connect Group | A youth-specific connect group within the ZOE Youth ministry — from yth.ttl |
| `YthNight` | Yth Night | A scheduled ZOE Youth ministry night event — from yth.ttl |
| `ZoeEvent` | ZOE Event | A scheduled church event (service, conference, workshop) at a ZOE LA campus (canonical IRI). |
| `ZoeKidsSession` | ZOE Kids Session | A scheduled ZOE Kids ministry session for children — from kids.ttl |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `activatesRoute` | PersonProfile | RouteInstance | A person's action (need submission, event registration, etc.) activates a Servic |
| `assignmentSatisfiesStage` | VolunteerAssignment | RouteStage | A confirmed volunteer assignment satisfies the role requirement for a route stag |
| `consentUnlocksStage` | Consent | RouteStage | A granted consent record unlocks the corresponding gated route stage. |
| `eventActivatesRoute` | ZoeEvent | RouteInstance | An event (e.g. a baptism class) activates the corresponding service route instan |
| `evidenceGeneratesReceipt` | Evidence | ServiceReceipt | Completed evidence triggers generation of a ServiceReceipt for the route stage. |
| `memberOfHousehold` | PersonProfile | Household | A person is a member of a household grouping. |
| `needActivatesRoute` | Need | ServiceRoute | A Need activates the appropriate ServiceRoute for the need type. |
| `needTriggersReferral` | Need | Referral | An unmet or partially-met need may trigger an external referral. |
| `outcomeEmitsEvent` | RouteOutcome | OcelEvent | A route outcome resolution emits an OCEL event for process mining. |
| `personInSegment` | PersonProfile | Concept | Links a person to their generation segment (affects consent requirements and min |
| `personRegisteredFor` | PersonProfile | EventRegistration | Links a person to their event registration records. |
| `receiptEmitsOcelEvent` | ServiceReceipt | OcelEvent | A service receipt triggers an OCEL event for process mining. |
| `referralTargetsPartner` | Referral | PartnerOrganization | A referral is directed to a specific partner organization. |
| `registrationForEvent` | EventRegistration | ZoeEvent | Links an EventRegistration record to the ZoeEvent it registers for. |
| `routeHasOutcome` | RouteInstance | RouteOutcome | Links a completed route instance to its recorded outcome. |
| `stageRequiresEvidence` | RouteStage | Evidence | A route stage requires evidence before it can be marked complete. |
