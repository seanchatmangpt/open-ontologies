# generations.ttl

_Generated 2026-05-18T19:57:14Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/zoela/generations.ttl`
- **Triples:** 263
- **Classes:** 2 · **Properties:** 12 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `FaithMilestone` | Faith Milestone | A significant faith event or sacramental action in a person's spiritual journey at ZOE LA. |
| `GenerationSegment` | Generation Segment | A named age-based ministry segment representing a generation cohort within ZOE LA. |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `ageRangeMax` | GenerationSegment | integer | Maximum age (inclusive) in years for this generation segment; use 999 to indicat |
| `ageRangeMin` | GenerationSegment | integer | Minimum age (inclusive) in years for this generation segment. |
| `completionReceiptType` | FaithMilestone | string | The receipt type code issued upon completion of this faith milestone (e.g. Bapti |
| `defaultConsentRequired` | GenerationSegment | string | The consent code required by default for individuals in this generation segment  |
| `milestoneCode` | FaithMilestone | string | Short uppercase code uniquely identifying this faith milestone (e.g. BAPTISM, CO |
| `milestoneLabel` | FaithMilestone | string | Human-readable display name for this faith milestone. |
| `ministryCode` | GenerationSegment | string | Identifier of the primary ministry responsible for serving this generation segme |
| `requiresEvent` | FaithMilestone | boolean | True when this faith milestone must be fulfilled in conjunction with a scheduled |
| `requiresGuardian` | GenerationSegment | boolean | True when individuals in this segment must be accompanied or consented to by a l |
| `requiresRegistration` | FaithMilestone | boolean | True when formal registration is required before completing this faith milestone |
| `segmentCode` | GenerationSegment | string | Short uppercase code identifying this generation segment (e.g. INFANT, PREK, ELE |
| `targetSegment` | FaithMilestone | Concept | The primary generation segment for which this faith milestone is intended. |
