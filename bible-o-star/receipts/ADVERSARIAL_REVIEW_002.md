# ADVERSARIAL_REVIEW_002 — Dung Gate Adversarial Review
**Reviewer:** Agent 8 — Dung Gate (Malkijah / Adversarial)
**Date:** 2026-06-02
**Scope:** All files in /Users/sac/open-ontologies/bible-o-star

---

## Summary Counts

| Severity | Count |
|---|---|
| CRITICAL | 4 |
| MAJOR | 7 |
| MINOR | 5 |

**Overall verdict: PARTIAL** — Structural foundations are sound. Multiple defects require remediation before ALIVE can be sustained.

---

## Findings Table

| # | Finding | Severity | File | Line (approx) | Fix Required |
|---|---|---|---|---|---|
| C1 | README and BIBLE_O_STAR_001.md declare **"Status: ALIVE"** but CELL8_CONFORMANCE_RECEIPT shows **"Overall Cell8 Verdict: PARTIAL"** with 5 failing gates (A5 Prove, A6 Seal, A8 Journal, A11 Governance, A12 Rollback). ALIVE is claimed without a sealed InspectionReceipt that passes all gates. This is a `bos:SelfCertifiedALIVE` defect — the exact anti-pattern the ontology prohibits. | CRITICAL | `README.md`, `BIBLE_O_STAR_001.md` | README line 9, BIBLE_O_STAR_001.md line 6 | Downgrade README status to PARTIAL until Cell8 gates A5/A6/A8/A11/A12 close. Remove ALIVE claim or qualify it as "Inspection Gate narrative verdict pending Cell8 certification." |
| C2 | SHACL validation receipt claims **"SHACL Conformance: TRUE"** without qualification, but the validation script (`validate_bible_o_star.sh`) only runs pyshacl against `nehemiah-52.ttl` — an ontology file with **zero instances of any shaped class**. The conformance is vacuously true. All `examples/*.ttl` instances were excluded from SHACL validation. Multiple example instances fail required SHACL properties (see M1, M2). | CRITICAL | `receipts/BIBLE_O_STAR_001_VALIDATION_RECEIPT.md`, `scripts/validate_bible_o_star.sh` | Receipt line 11; script pyshacl call | Extend SHACL validation to include all `examples/*.ttl` as the data graph. Fix shape violations before re-claiming conformance. |
| C3 | **Nehemiah 3:13 says "one thousand cubits" (אֶלֶף אַמָּה), not 500.** Four documentation files systematically misquote this as "500 cubits," inverting the textual evidence. The Valley Gate section is used throughout the operating grammar as the canonical lower-bound reference — citing the wrong measurement corrupts every derived claim that references it. Note: `examples/valley-gate-record.ttl` correctly says "one thousand cubits," making the docs-vs-TTL inconsistency itself a structural defect. | CRITICAL | `docs/GATE_ASSIGNMENT_MODEL.md` line 37, `docs/BUILDER_REGISTRY.md` lines 32/72/73, `docs/NEHEMIAH_52_OPERATING_GRAMMAR.md` lines 317/329/337/339/554, `docs/WALL_SECTION_REGISTRY.md` line 13 | Multiple | Replace all instances of "500 cubits" (for Neh.3.13) with "1000 cubits" or "one thousand cubits" in all four doc files. |
| C4 | **`bos:assignedTo` is used in `examples/fish-gate-landing-page.ttl` but is never declared in any ontology file.** This is an undeclared property — any reasoner or SPARQL query relying on it will silently fail or produce no bindings. This is distinct from `bos:assignedToGate`, which is correctly declared. | CRITICAL | `examples/fish-gate-landing-page.ttl` line 26 | `bos:FishGate bos:assignedTo bos:FishGateSection` | Either remove the triple or replace with the declared `bos:hasWallSection` inverse pattern. |
| M1 | **All `bos:InspectionReceipt` instances in examples are missing `bos:hasCanonicalReference`** (required `sh:minCount 1` by `bos:InspectionReceiptShape`). Affected files: `sheep-gate-record.ttl`, `dung-gate-record.ttl`, `old-gate-record.ttl`, `inspection-gate-receipt.ttl`, `water-gate-record.ttl`, `fountain-gate-record.ttl`, `valley-gate-record.ttl`, `horse-gate-record.ttl`, `east-gate-record.ttl`, `courier-false-report-record.ttl`, `fish-gate-landing-page.ttl`. This is the same SHACL violation hidden by C2. | MAJOR | All `examples/*.ttl` files except `water-gate-pericope.ttl` | Multiple | Add `bos:hasCanonicalReference "Neh.X.Y"^^xsd:string` to every `bos:InspectionReceipt` instance in all example files. |
| M2 | **Both `bos:MusterLedgerRecord` instances in `muster-ledger-record.ttl` are missing `bos:hasCanonicalReference` and `bos:assignedToGate`**, both required by `bos:MusterLedgerRecordShape`. The instances use `bos:hasMusterRecord` (pointing to a `bos:MusterRegistry`, an undefined class) instead of the required property structure. | MAJOR | `examples/muster-ledger-record.ttl` lines 20-32 | Multiple | Add `bos:hasCanonicalReference` and `bos:assignedToGate` to both `bos:MusterRecord001` and `bos:MusterRecord002`. |
| M3 | **`bos:MusterRegistry` and `bos:UsuryAudit` are used as RDF types in examples but are never declared as classes in any ontology file.** `bos:MusterRegistry001 a bos:MusterRegistry` and `bos:UsuryAudit001 a bos:UsuryAudit` both reference undefined classes. A reasoner will treat these as `owl:Thing` with no constraint, silently accepting invalid instances. | MAJOR | `examples/muster-ledger-record.ttl` line 34, `examples/usury-ledger-record.ttl` line 22 | As cited | Declare `bos:MusterRegistry` and `bos:UsuryAudit` as `owl:Class` in `nehemiah-52.ttl`, or remove these individuals and replace with the correct declared types. |
| M4 | **`bos:hasTimestamp` is used in `inspection-gate-receipt.ttl` but is never declared in any ontology file.** `bos:InspectionReceipt52Days bos:hasTimestamp "445-09-25"^^xsd:string` is a dangling property triple. Any application relying on timestamps will not find this property in the ontology schema. | MAJOR | `examples/inspection-gate-receipt.ttl` line 45 | As cited | Declare `bos:hasTimestamp a owl:DatatypeProperty` with `rdfs:range xsd:string` in `nehemiah-52.ttl`, or use the standard `dcterms:date` property. |
| M5 | **`bos:InspectionReceipt52Days` has `bos:hasReceipt bos:InspectionReceipt52Days` — the receipt points to itself.** A self-referential receipt chain is not a valid provenance chain; it is an infinite loop. This violates the receipt grounding requirement. | MAJOR | `examples/inspection-gate-receipt.ttl` lines 43-44 | `bos:hasReceipt bos:InspectionReceipt52Days` on the receipt itself | Remove the self-referential `bos:hasReceipt` triple from `bos:InspectionReceipt52Days`. A receipt does not ground itself. |
| M6 | **TOGAF_MAPPING.md states its per-ADM-phase mappings are "normative — not illustrative"** without any TOGAF certification, TOGAF license, or reference to The Open Group documentation. TOGAF is a registered trademark of The Open Group. Claiming a mapping is "normative" implies official alignment with the TOGAF standard — a claim that requires evidence from The Open Group, not a solo mapping exercise. | MAJOR | `docs/TOGAF_MAPPING.md` line 24 | As cited | Change "normative — not illustrative" to "interpretive" or "structurally analogous." Add a disclaimer that TOGAF is a registered trademark of The Open Group and that this mapping is not certified by The Open Group. |
| M7 | **BUILDER_REGISTRY.md assigns Meremoth's second section to "Horse Gate"** (line 22), but Nehemiah 3:21 places his second section near Eliashib's house — geographically near the Sheep Gate / Fish Gate area on the north wall, not near the Horse Gate (Neh. 3:28) on the east wall. The TTL correctly assigns Meremoth to FishGate only, but the doc is wrong. | MAJOR | `docs/BUILDER_REGISTRY.md` line 22 | `Fish Gate (section); Horse Gate (second section)` | Change "Horse Gate (second section)" to "Fish Gate area (second section, Neh. 3:21)" to match the textual evidence and the TTL. |
| Mi1 | **`bos:InspectionGate` is used simultaneously as a class name (`owl:Class`) and as the IRI of an individual (`owl:NamedIndividual`).** In OWL DL, a class and an individual may not share the same IRI. This forces the ontology into OWL Full, which is not formally decidable and breaks OWL DL-based reasoners. | MINOR | `ontology/nehemiah-52.ttl` lines 115, 275 | As cited | Rename the individual to `bos:InspectionGateInstance` or `bos:TheInspectionGate`, preserving the class as `bos:InspectionGate`. |
| Mi2 | **`bos:hasSource` and `bos:assignedToGate` in `bible-o-star.ttl` are missing `rdfs:domain`**, and `bos:hasSource` is also missing `rdfs:range`. These properties are used broadly across examples with inconsistent subjects and objects (sometimes `bos:hasSource` takes a literal string, sometimes an IRI). The absence of domain/range means misuse is invisible to reasoners. | MINOR | `ontology/bible-o-star.ttl` lines 102-104, 129-132 | As cited | Add `rdfs:domain` and `rdfs:range` to `bos:hasSource`. At minimum, document the intentional open-world design decision in the comment. |
| Mi3 | **The word "generated" appears twice in `docs/GGEN_PIPELINE_NOTES.md`** — once in the pipeline description and once in a SPARQL comment about generated admission logic. The terminology policy prohibits "generated"; the canonical term is "composed" or "emitted." | MINOR | `docs/GGEN_PIPELINE_NOTES.md` lines 5, 146 | As cited | Replace "generated artifacts" with "composed artifacts" and "generated admission logic" with "emitted admission logic." |
| Mi4 | **LICENSE_AND_USAGE_BOUNDARY.md states "Current `dcterms:license` in `bible-o-star.ttl` and `nehemiah-52.ttl` is CC0 1.0"** but the actual files currently declare CC BY 4.0. The issue description is stale. The document carries a "PARTIAL verdict" header that was not updated after the fix was applied. | MINOR | `docs/LICENSE_AND_USAGE_BOUNDARY.md` lines 18-19 | As cited | Update the "Remaining Issues" section to reflect that the license mismatch was resolved. Upgrade the doc-level verdict to match the current state. |
| Mi5 | **`water-gate-pericope.ttl` assigns `bos:pericopeIndex 14` to the Baptism of Jesus pericope** without citing which pericope numbering scheme the value 14 belongs to. The CGI source (referenced via `dcterms:source`) uses its own numbering. Using a numeric value from CGI without explicit attribution to the CGI numbering system is ambiguous. | MINOR | `examples/water-gate-pericope.ttl` line 37 | `bos:pericopeIndex 14` | Add an `rdfs:comment` clarifying that index 14 is an independent assignment for illustration and does not directly map to CGI pericope #14 unless that alignment has been verified. |

---

## Fake Gate Audit

All 7 refused fake gates (`InterestGate`, `PeopleGate`, `MessengerGate`, `NationsGate`, `ProphetGate`, `RumorGate`, `ReportGate`) are present in `nehemiah-52.ttl` with `owl:deprecated true` and `rdfs:comment "REFUSED: not a gate."` — correctly handled. No active fake gate was found. The anti-pattern registry in `GATE_ASSIGNMENT_MODEL.md` is correctly populated.

**Fake gate verdict: PASS**

---

## Word "generated" Audit

The word "generated" appears in `docs/GGEN_PIPELINE_NOTES.md` lines 5 and 146 in the context of code generation pipeline description. These are terminology violations (see Mi3 above). No occurrence was found in any `.ttl` file.

**Word "generated" found: YES** (docs/GGEN_PIPELINE_NOTES.md only)

---

## IP Leakage Audit

No proprietary Bible translation text, no Lexham/Logos/Accordance material, and no `gall:` namespace vocabulary was found. The `bos:` namespace is consistently `https://open-ontologies.org/bible-o-star#` across all four ontology files (the earlier namespace split reported in IP_AUDIT_RESULT.md was resolved). CC BY-NC-SA 2.0 from the CGI source is correctly handled as "structural pattern only, no data copied."

No TOGAF license or trademark attribution is present — see M6.

**IP leakage verdict: CLEAN** (with M6 TOGAF trademark disclaimer gap)

---

## Overall Adversarial Verdict: PARTIAL

The ontology has sound structural foundations: Turtle parses cleanly, classes and properties are labeled, source provenance is documented, and the fake gate / deprecated individual handling is correct. The operating grammar documentation is coherent.

The defects are real and not cosmetic:

- The ALIVE self-declaration (C1) violates the ontology's own core law (`bos:SelfCertifiedALIVE`).
- The vacuous SHACL conformance claim (C2) hides at least 11+ SHACL violations across examples.
- The 1000-vs-500 cubit error (C3) corrupts the canonical lower-bound reference throughout the grammar documentation.
- Two undeclared properties/classes and one self-referential receipt demonstrate that the examples were not validated against the shapes they claim to instantiate.

None of these are fatal to the ontology itself, but they must be resolved before ALIVE can be honestly claimed.
