# License and Usage Boundary — Bible O*

**Gate:** Old Gate (Joiada) — IP Boundary Inspection  
**Composed:** 2026-06-02  
**Verdict:** PARTIAL (see Remaining Issues below)

---

## Ontology License

**Recommendation: CC BY 4.0 International**

The Bible O* ontology package (namespace, class hierarchy, property set, SHACL shapes, Nehemiah grammar, and examples) is released under the **Creative Commons Attribution 4.0 International License (CC BY 4.0)**.

License URI: <https://creativecommons.org/licenses/by/4.0/>

Attribution: "Bible O* Ontology — Open Ontologies Project"

Note: The current `dcterms:license` declarations in `bible-o-star.ttl` and `nehemiah-52.ttl` reference CC0 1.0. The recommendation here is CC BY 4.0 to align with the OSHB upstream attribution requirement and to ensure downstream consumers credit the Open Scriptures Hebrew Bible Project as required by their CC BY 4.0 license. File-level license declarations should be updated in a future pass.

---

## What Is Public

The following are declared public and may be used, modified, and distributed under CC BY 4.0 with attribution:

| Component | Status | Notes |
|---|---|---|
| `bos:` namespace (`https://open-ontologies.org/bible-o-star#`) | Public | The canonical IRI space for all Bible O* terms |
| Ontology class hierarchy | Public | All classes in `bible-o-star.ttl` and `nehemiah-52.ttl` |
| Property set | Public | All object and datatype properties |
| SHACL shapes (`nehemiah-52-shapes.ttl`) | Public | Validation shapes for gate, builder, receipt, etc. |
| Nehemiah grammar doctrine | Public | All 10 canonical gate instances; refused fake gate declarations |
| Example instances | Public | All `.ttl` files in `examples/` |
| Documentation (`docs/`) | Public | Operating grammar, TOGAF mapping, gate model, courier model, etc. |

---

## What Is Private

The following are NOT part of this public ontology package:

| Component | Status | Notes |
|---|---|---|
| Operational receipts from specific deployments | Private | Receipts from a given organization's CodeManufactory run are operational secrets |
| Adaptation-specific builder assignments | Private | Which specific agent or team is assigned to which gate in a deployment is an implementation detail |
| Organization-specific proof gate criteria | Private | What constitutes admission at a specific org's DungGate or SheepGate is not disclosed in the ontology |
| CodeManufactory pipeline internals | Private | The operational pipeline that manufactures artifacts is a separate system — not part of this ontology |

---

## Explicit Statement: No Proprietary Bible Translation Text

This ontology package contains **no copyrighted Bible translation text**.

- No ESV, NIV, NASB, NLT, or other modern translation text appears in any file.
- No Lexham Bible Dictionary content.
- No Logos proprietary dataset.
- No Crossway, Tyndale, or other publisher-owned material.

The only textual references to scripture are:
1. OSIS canonical reference strings (e.g. `"Neh.3.1"`) — these are addresses, not translation text.
2. Brief descriptive comments citing the scriptural source (e.g. `bos:hasSource "Neh.3.3"`) — these are citations, not quotations.

---

## Source License Summary

| Source | License | Attribution Required | Usage in Bible O* |
|---|---|---|---|
| OSIS (Open Scripture Information Standard) | CC BY-SA 4.0 | Yes | Reference model: canonical address syntax only |
| OSHB lemma/morphology data | CC BY 4.0 | Yes — "Open Scriptures Hebrew Bible Project" | Reference model: no text copied |
| OSHB WLC base text | Public Domain | No | Not copied |
| OpenBible.info Cross References | CC BY 4.0 | Yes — "OpenBible.info" | Relation model: no data copied |
| Composite Gospel Index RDF | CC BY-NC-SA 2.0 | Yes (NC restriction on data) | Structural pattern only (inspired_by); no data copied |
| W3C RDF/RDFS/OWL/SHACL/DCTERMS/SKOS | W3C Document License | Per W3C terms | Standard vocabulary — standard usage |

---

## Hard Prohibitions

The following patterns are permanently prohibited and must be refused at the Old Gate:

1. Any `gall:` prefix vocabulary in any file — refused.
2. Any proprietary namespace (Logos, Lexham, Crossway, etc.) used as the ontology foundation — refused.
3. Any copyrighted Bible translation text reproduced in any file — refused.
4. Any fake gate (`InterestGate`, `PeopleGate`, `MessengerGate`, `NationsGate`, `ProphetGate`, `RumorGate`, `ReportGate`) declared as a live `bos:Gate` instance — refused.
5. Any non-canonical gate instance (e.g. `MusterGate`) declared as `bos:Gate` — refused.
6. Any ALIVE verdict claimed without a sealed `bos:InspectionReceipt` — refused.

---

## Remaining Issues (PARTIAL verdict)

The following issues require resolution before this package achieves full ALIVE status at the Old Gate:

1. **Namespace split (HIGH):** `bible-o-star.ttl` and `source-ledger.ttl` use `http://open-ontologies.org/ontology/bible-o-star#` (HTTP, with extra `/ontology/` path segment). The canonical namespace is `https://open-ontologies.org/bible-o-star#` (HTTPS, no `/ontology/` segment). These two files must be updated to align with `nehemiah-52.ttl` and all example files.

2. **MusterGate defect (HIGH):** `examples/mocker-feedback-record.ttl` declares `bos:MusterGate a bos:Gate` — a non-canonical gate instance not among the 10 sanctioned gates. This must be removed. `bos:assignedToGate` references to `bos:MusterGate` must be rerouted to `bos:InspectionGate` or the `bos:MusterLedger` individual as appropriate.

3. **License declaration mismatch (LOW):** Current `dcterms:license` in `bible-o-star.ttl` and `nehemiah-52.ttl` is CC0 1.0. This document recommends CC BY 4.0 to align with OSHB attribution requirements. The license URIs in those files should be updated.

4. **source-ledger.ttl namespace** was using `http://open-ontologies.org/ontology/bible-o-star#` — corrected in this audit pass to `https://open-ontologies.org/bible-o-star#`.
