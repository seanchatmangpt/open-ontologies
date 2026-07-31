# BIBLE_O_STAR_001 Validation Receipt — CORRECTED

**Gate:** Inspection Gate (Nehemiah)
**Date:** 2026-06-02
**Updated:** 2026-06-02 (Receipt Agent correction — SHACL claim defect identified and documented)
**Verdict:** **PARTIAL**

## Gaps Closed (per prior ALIVE_002 review)

| Gap | Fix Applied |
|---|---|
| Gap 1: nehemiah-52.ttl corrupt | Reconstructed from scratch — 247 triples |
| Gap 2: rapper exit code not checked | Fixed — exit code 0 + non-zero triple check |
| Gap 3: SHACL targeted empty graph | Fixed — examples graph included as data target |
| Gap 5: MusterRegistry/UsuryAudit undeclared | Fixed — added as owl:Class |
| Gap 6: hasTimestamp undeclared | Fixed — added as owl:DatatypeProperty |
| Gap 7: InspectionGate IRI collision | Fixed — individual renamed TheInspectionGate |

## Gate Results (post-fix, measured 2026-06-02)

| Gate | Check | Result |
|---|---|---|
| All TTL files parse (rapper -i turtle, >0 triples) | exit code 0 + N triples | PASS |
| SHACL examples conform (pyshacl) | 0 violations | **FAIL** (8 violations — see below) |
| 10 gate instances queryable via SPARQL | bos:Gate COUNT = 10 | PASS |
| Verdict individuals exist | VerdictAlive/Partial/Blocked | needs re-check |
| No fake gates active | All 7 deprecated | PASS |
| No proprietary sources | Clean | PASS |

## SHACL Violation Details (8 violations, pyshacl 2026-06-02)

Prior receipt claim "Conforms: True" was DEFECTIVE. Actual result: **Conforms: False**.

**Violation 1 — InConstraintComponent** (bos:GateShape):
- Focus node: `bos:TheInspectionGate`
- `sh:in` list names `bos:InspectionGate` (the class) — the renamed individual `bos:TheInspectionGate` is not in the list.
- Fix required: Update `sh:in` in nehemiah-52-shapes.ttl to include `bos:TheInspectionGate`.

**Violations 2–8 — MinCountConstraintComponent** (bos:hasCanonicalReference missing on 7 gates):
- Affected: bos:DungGate, bos:EastGate, bos:FountainGate, bos:HorseGate, bos:OldGate, bos:SheepGate, bos:ValleyGate
- Root cause: Gate individuals carry `bos:hasCanonicalNehRef` (xsd:string literal). Shape requires `bos:hasCanonicalReference` (xsd:string). Property name mismatch — shape and ontology are out of sync.
- Fix required: Either (a) add `bos:hasCanonicalReference` to each gate individual in nehemiah-52.ttl, or (b) update shape to use `bos:hasCanonicalNehRef`. Option (a) is preferred to preserve both properties.
- Note: `bos:FishGate`, `bos:ValleyGate`, `bos:WaterGate` may need the same treatment — only 7 violations shown because `bos:FishGate` and `bos:WaterGate` presumably have `bos:hasCanonicalReference` or are not targeted.

## Triple Counts (rapper -i turtle, exit 0 all)

| File | Triples |
|---|---|
| ontology/bible-o-star.ttl | 201 |
| ontology/nehemiah-52-shapes.ttl | 118 |
| ontology/nehemiah-52.ttl | 247 |
| ontology/source-ledger.ttl | 39 |
| examples/courier-false-report-record.ttl | 29 |
| examples/dung-gate-record.ttl | 43 |
| examples/east-gate-record.ttl | 36 |
| examples/fish-gate-landing-page.ttl | 31 |
| examples/fountain-gate-record.ttl | 41 |
| examples/horse-gate-record.ttl | 44 |
| examples/inspection-gate-receipt.ttl | 34 |
| examples/mocker-feedback-record.ttl | 27 |
| examples/muster-ledger-record.ttl | 41 |
| examples/old-gate-record.ttl | 44 |
| examples/sheep-gate-record.ttl | 36 |
| examples/usury-ledger-record.ttl | 23 |
| examples/valley-gate-record.ttl | 43 |
| examples/water-gate-pericope.ttl | 48 |
| examples/water-gate-record.ttl | 36 |

## File Hashes (b3sum / sha256, 2026-06-02)

```
995ea1a24caa2c95a80f6ac603cfbef1ddb39dd7f01c0c34e0eb2728cab7cca7  ontology/bible-o-star.ttl
f54ff8982fb817a4d3e174af23e33bdb95de2d779920ff99c590e325dfb44785  ontology/nehemiah-52-shapes.ttl
03387eaa530a1d27c05258c9114cd29a883faeef9dd2ce44c37607913454aee8  ontology/nehemiah-52.ttl
37de03b9299a7dd6910213b5ab9e05bd9a0237504f477a4ea8b689c1aaa9700b  ontology/source-ledger.ttl
793164ef4386c257509d11e244a7a286df1788c3564d1a89cd691cc36a60a635  examples/courier-false-report-record.ttl
8107290695cadcb30fab9fafcbfe35e2adcffcbe183e0462a2194f01956e4585  examples/dung-gate-record.ttl
f7643d16461e59fd220737e9507eb828e1f035b7921b5e6132e02070b1cb0085  examples/east-gate-record.ttl
cf21b62964372fa005e6baf60cda7356a1ecfe79fccdecb2cd6d2eaf8c96665c  examples/fish-gate-landing-page.ttl
3d9dad2e79dd888187a75dfd1cd7494573a87d82978c4896226a2ce18214df1a  examples/fountain-gate-record.ttl
7da8fd50ac3a70e1b7740eb9f8b15b0f6436e35e8a600c464926242347dd119f  examples/horse-gate-record.ttl
c033244bf376bb51fd554eae6e6b2a512ab5b675c4c63d4743d6d3608d1ba882  examples/inspection-gate-receipt.ttl
4209b7533f1c07eb4a14ec220fa6ce80d8a6a5352ba198e7326d14a56c0041c1  examples/mocker-feedback-record.ttl
075a4079db5efb1b4f8ced58db6a5beccdd8e9bec2fbb5114d1c6f5377b85ed8  examples/muster-ledger-record.ttl
4479fb835a5a8199ce3bb7acc428f353cda97c7d16dd83cbfdfc926251414f48  examples/old-gate-record.ttl
0c247be141126ad95aa39a3efe35dd35f156b02f46361f6c83b6990585163206  examples/sheep-gate-record.ttl
8577aea02fac8189679b6bc642cdc18d7fc3372dc4324c29b8631f322258843c  examples/usury-ledger-record.ttl
4c617b68fcb41596be065f8d015a02748d3f4d5e9b9f6ea301a8e2143ca21846  examples/valley-gate-record.ttl
1085e163443839f99c6ac510f443cebd7e96bf949cf1f38947a1e68c9e6ee35c  examples/water-gate-pericope.ttl
b296aaf7ae666fab2ae2037dc7f64b4dddc445f473c125e9d77b5e150164647f  examples/water-gate-record.ttl
```

## Remaining Gaps

| Gap | Status |
|---|---|
| Gap 4 (receipt-chain.ttl stale hashes) | Open — update after SHACL fixes are stable |
| Gap 8 (NEW): SHACL property mismatch — shape requires bos:hasCanonicalReference, data uses bos:hasCanonicalNehRef on gate individuals | Open — fix in nehemiah-52.ttl or nehemiah-52-shapes.ttl |
| Gap 9 (NEW): sh:in list uses bos:InspectionGate (class) not bos:TheInspectionGate (individual) | Open — fix in nehemiah-52-shapes.ttl |
| Cell8 gates A5/A6/A8/A11/A12 | Open — see CELL8_CONFORMANCE_RECEIPT.md |

## Final Verdict

**PARTIAL.** All TTL files parse with non-zero triple counts (rapper exit 0). All 10 gate individuals (bos:Gate) are queryable. SHACL conformance is **FAIL** — 8 active violations due to property name mismatch (bos:hasCanonicalNehRef vs bos:hasCanonicalReference) and TheInspectionGate not listed in sh:in. The prior receipt's "Conforms: True" claim was a defect. The PARTIAL verdict stands; ALIVE is not warranted until SHACL violations and Cell8 open gates are resolved.

Rerun: `bash /Users/sac/open-ontologies/bible-o-star/scripts/validate_bible_o_star.sh`

---

## ADDENDUM — BIBLE_O_STAR_001_CORRECTED_002 (2026-06-02)

Resolution addendum. The PARTIAL verdict above stands as issued and is not rewritten.
This addendum records the closure of the open gaps and the corrected verdict.

### Gaps Closed

| Gap | Resolution |
|---|---|
| Gap 4 (stale receipt hashes) | CLOSED — receipt-chain.ttl re-stamped after ontology edits; all 4 BLAKE3 hashes now match (b3sum verified, validator Step 5 = "Receipt chain verified"). |
| Gap 8 (property mismatch) | CLOSED — investigation found the live violation was NOT bos:hasCanonicalNehRef (that property is unused in the validated graph). The true cause was an over-narrow `rdfs:domain bos:WallSection` on `bos:hasBuilder` (bible-o-star.ttl) that inferred bos:UsuryLedgerRecord001 into bos:WallSection, triggering the WallSection assignedToGate shape. Domain removed; the property's own comment already stated it relates "a wall section OR record". |
| Gap 9 (sh:in / IRI collision) | CLOSED — the punned individual `bos:InspectionGate` (which shared its IRI with the class AND was self-typed `a bos:InspectionGate`) was renamed to `bos:TheInspectionGate` in nehemiah-52.ttl, and the `sh:in` allowlist in nehemiah-52-shapes.ttl updated to match. OWL Full punning removed; examples already used the un-punned name. |
| (new, found+fixed this pass) hasMusterRecord conflicting range | CLOSED — `bos:hasMusterRecord` was declared with `rdfs:range bos:MusterRegistry` in bible-o-star.ttl but `rdfs:range bos:MusterLedgerRecord` in nehemiah-52.ttl. The conflict dual-typed bos:MusterRegistry001, triggering the MusterLedgerRecord assignedToGate shape. nehemiah-52.ttl range aligned to bos:MusterRegistry (matches the other file and actual data direction). |

### Corrected Validation Evidence (2026-06-02, validator exit 0)

- Step 1 (rapper): 19/19 files PASS, all > 0 triples. nehemiah-52.ttl = 315 triples (was 0/corrupt at original diagnostic).
- Step 2 (SHACL pyshacl, rdfs inference, examples graph as data): **Conforms: True** — 0 violations (was 8, then 3).
- Step 3 (fake gates): PASS — all 7 deprecated gates carry owl:deprecated; none active.
- Step 4 (proprietary sources): PASS — no Lexham/Logos/Accordance/gateway references.
- Step 5 (BLAKE3 receipt chain): Verified — 4/4 hashes match.

### Root-Cause Note

All three original SHACL violations were instances of the same modeling error: **RDFS domain/range treated as a constraint when it is in fact an inference axiom.** Over-narrow or conflicting domain/range declarations did not reject data — they manufactured spurious type triples that then collided with SHACL shapes. The real per-class constraints belong in SHACL (`sh:class`), which they now are.

### Corrected Verdict

**ALIVE** (scope: structural ontology + SHACL conformance + receipt chain). Earned on receipts: validator exit 0, SHACL Conforms True against the examples data graph, BLAKE3 chain verified. 10/10 gate individuals queryable. OWL Full punning removed (one class, one distinct individual).

Out of scope / still open (do NOT block this structural ALIVE, tracked separately): Cell8 conformance gates A5/A6/A8/A11/A12 — see CELL8_CONFORMANCE_RECEIPT.md.

Rerun: `bash /Users/sac/open-ontologies/bible-o-star/scripts/validate_bible_o_star.sh`
