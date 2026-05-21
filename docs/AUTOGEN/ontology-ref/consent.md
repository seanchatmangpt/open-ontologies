# consent.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/consent.ttl`
- **Triples:** 176
- **Classes:** 2 · **Properties:** 12 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `Consent` | Consent | A recorded consent granted by or on behalf of a person for a specific purpose within the ZOE LA Ministry. Subclass of sc |
| `ConsentRequirement` | Consent Requirement | A gate condition that blocks a named route stage until an active Consent of the required type exists for the target pers |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `blocksRouteStage` | ConsentRequirement | string | Identifier of the route stage (e.g., 'youth_camp_enroll', 'partner_referral_send |
| `consentCode` | Consent | string | Short uppercase code uniquely identifying the consent type; matches the correspo |
| `consentExpiresAt` | Consent | dateTime | Optional ISO-8601 timestamp after which the consent is no longer valid. Null mea |
| `consentIsActive` | Consent | boolean | Boolean flag. False when the consent has been explicitly revoked or has passed i |
| `consentType` | Consent | Concept | Links a Consent record to the SKOS ConsentType concept that classifies it. |
| `grantedAt` | Consent | dateTime | ISO-8601 timestamp at which the consent was recorded as granted. |
| `grantedBy` | Consent | string | UUID of the person who granted consent. For guardian consent this is the guardia |
| `grantedFor` | Consent | string | UUID of the subject person for whom consent is being granted. Differs from grant |
| `hasDocument` | Consent | string | URL of the signed consent document stored in object storage (e.g., Supabase Stor |
| `isGuardianConsent` | Consent | boolean | When true, the consent was granted by a parent or legal guardian on behalf of a  |
| `refusalMessageCode` | ConsentRequirement | string | i18n message code returned to the client when this consent requirement blocks a  |
| `requiresConsentType` | ConsentRequirement | Concept | The SKOS ConsentType concept that must be present and active before the blocked  |
