# IP Audit Result — BIBLE_O_STAR_001

**Audit Date:** 2026-06-02
**Auditor:** Agent 9 (Joiada / Old Gate) — Source, License, and IP Boundary Inspector
**Scope:** All files in /Users/sac/open-ontologies/bible-o-star

---

## Proprietary material check: PASS

No proprietary text from Lexham, Logos, Accordance, or any publisher bearing
"all rights reserved" or "copyrighted text" markers was found in any file.

The words "Lexham" and "Logos" appear only in `docs/LICENSE_AND_USAGE_BOUNDARY.md`
and `docs/PUBLIC_SOURCE_LEDGER.md` as explicit exclusion statements — confirming
that these sources are ruled out, not incorporated.

The validation script `scripts/validate_bible_o_star.sh` includes a proprietary
content grep guard confirming this boundary is actively enforced.

---

## Fake gate check: PASS

All seven refused fake gates (InterestGate, PeopleGate, MessengerGate, NationsGate,
ProphetGate, RumorGate, ReportGate) appear only in:

- `README.md` — under the heading "Refused Fake Gates," explicitly labelled deprecated anti-patterns
- `docs/LICENSE_AND_USAGE_BOUNDARY.md` — listed as prohibited constructs
- `docs/MUSTER_LEDGER.md` — PeopleGate and MessengerGate appear under refusal headings
- `docs/PROPHETIC_PROCLAMATION_MODEL.md` — ProphetGate appears under an explicit "No ProphetGate" section
- `ontology/nehemiah-52.ttl` — marked deprecated in the ontology

No file instantiates any fake gate as a real, non-deprecated ontology class or
individual. All occurrences are refusals, not usages.

---

## gall: vocabulary check: PASS

No `gall:` prefix, term, or namespace declaration was found in any `.ttl` file or
any other file in the repository. The term "gall:" appears only in prose exclusion
lists in `BIBLE_O_STAR_001.md`, `docs/LICENSE_AND_USAGE_BOUNDARY.md`, and
`docs/PUBLIC_SOURCE_LEDGER.md` — confirming exclusion, not inclusion.

---

## Namespace consistency: WARN

Two distinct `bos:` namespace URIs are in use across the repository:

| Namespace URI | Files |
|---|---|
| `http://open-ontologies.org/ontology/bible-o-star#` | `ontology/bible-o-star.ttl`, `ontology/source-ledger.ttl`, `README.md` |
| `https://open-ontologies.org/bible-o-star#` | `ontology/nehemiah-52.ttl`, `ontology/nehemiah-52-shapes.ttl`, all 6 example `.ttl` files, `docs/NEHEMIAH_52_OPERATING_GRAMMAR.md` |

These are two different IRIs. A reasoner or SPARQL query using one namespace will
not unify with terms declared in the other. This is a namespace split defect.

**Recommended remediation:** Canonicalize to one URI across all files. The primary
ontology file (`bible-o-star.ttl`) uses the `http://` form; the majority of TTL
files use the `https://` form. Select one and update all declarations.

---

## Source ledger completeness: PASS

`ontology/source-ledger.ttl` contains 4 declared sources:

| Source | IRI |
|---|---|
| OSIS (Open Scripture Information Standard) | `bos:OSIS_Source` |
| Open Scriptures Hebrew Bible (OSHB) | `bos:OSHB_Source` |
| OpenBible.info Cross References | `bos:OpenBibleCrossReferences` |
| Composite Gospel Index RDF Pattern (semanticbible.com) | `bos:CompositeGospelIndex` |

All four required markers (OSIS, openscriptures, openbible, semanticbible) are
present — confirmed by grep count of 5 (one term has two hits).

The source ledger carries a CC0 1.0 license declaration, consistent with the
public-domain / open-license posture of the sourced datasets.

---

## Financial/trading language check: PASS

No references to profit (financial instrument), trading, capital deployment,
exchange API, or broker in the sense of financial brokerage were found in `docs/`.

The word "broker" appears twice in `docs/USURY_LEDGER.md` in a biblical/narrative
context: describing the characters in Nehemiah 5 who extracted interest from
builders. This is domain-appropriate use, not a financial product claim.

---

## Overall IP verdict: WARN

| Check | Result |
|---|---|
| Proprietary material (Lexham, Logos, Accordance) | PASS |
| Fake gate instantiation | PASS |
| gall: vocabulary | PASS |
| Namespace consistency | WARN — two distinct bos: URIs in use |
| Source ledger completeness | PASS |
| Unsupported financial/trading claims in docs | PASS |

**Verdict: WARN**

The repository is clean of proprietary material, fake gate instantiation, and
prohibited vocabulary. The single open issue is the namespace split between
`http://open-ontologies.org/ontology/bible-o-star#` and
`https://open-ontologies.org/bible-o-star#`. This is a structural defect that
does not constitute an IP violation but will cause semantic fragmentation under
linked-data tooling. It must be resolved before the ontology is published or
federated with other open-ontologies assets.

No FAIL conditions were found. The WARN is solely the namespace inconsistency.
