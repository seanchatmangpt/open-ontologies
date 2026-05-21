# referrals.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/referrals.ttl`
- **Triples:** 234
- **Classes:** 1 · **Properties:** 16 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `Referral` | Referral | A formal referral from ZOE LA to a partner organization for a person or household in need. Referrals require consent and |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `expectedResponseDays` | Referral | integer | The number of days within which a response from the partner organization is expe |
| `followUpRequired` | Referral | boolean | Indicates whether a follow-up action is required after this referral is complete |
| `initiatedAt` | Referral | dateTime | The date and time when this referral was initiated. |
| `initiatedBy` | Referral | string | The user ID of the ZOE LA staff member who initiated this referral. |
| `outcomeNote` | Referral | string | A note describing the outcome of this referral after it has been completed, refu |
| `partnerConfirmedAt` | Referral | dateTime | The date and time when the partner organization confirmed receipt and acceptance |
| `partnerId` | Referral | string | The identifier of the partner organization receiving this referral. |
| `receiptId` | Referral | string | A foreign key to the PartnerReferralReceipt record that provides cryptographic p |
| `referralConsentId` | Referral | string | A required foreign key to the consent record authorizing this referral. Referral |
| `referralId` | Referral | string | A unique identifier for this referral record. |
| `referralNote` | Referral | string | Pastoral or care notes associated with this referral. Access is sensitivity-gate |
| `referralStatus` | Referral | Concept | The current lifecycle status of this referral, drawn from the zoe:ReferralStatus |
| `referralType` | Referral | Concept | The type of service this referral addresses, drawn from the zoe:ReferralTypeSche |
| `sensitivityLevel` | Referral | string | The data sensitivity classification of this referral. Valid values: Public, Sens |
| `subjectId` | Referral | string | The identifier of the person or household being referred to the partner organiza |
| `subjectType` | Referral | string | The type of subject being referred, either 'person' or 'household'. |
