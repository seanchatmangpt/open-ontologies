# Public Source Ledger — Bible O*

## Purpose
This ledger records every public source used in composing the Bible O* ontology package.
No proprietary dataset was ingested. No copyrighted Bible translation text was copied.

## Source Entries

### 1. OSIS — Open Scripture Information Standard
- **What it provides:** OSIS provides a public XML schema / canonical reference layer for Bible texts and related research materials, including book/chapter/verse addressing.
- **License:** Open standard; XML schema is public.
- **How used in Bible O*:** Foundation for bos:ScriptureWork, bos:Book, bos:Chapter, bos:Verse, bos:Passage, bos:hasCanonicalReference. Uses OSIS three-letter abbreviations and period-delimited syntax (e.g., 'Neh.3.1').
- **Source URL:** https://crosswire.org/osis/
- **Wikipedia:** https://en.wikipedia.org/wiki/Open_Scripture_Information_Standard
- **Usage boundary:** Reference layer only. No OSIS text content was copied.

### 2. Open Scriptures Hebrew Bible (OSHB)
- **What it provides:** Open Scriptures Hebrew Bible is OSIS XML and includes lemma/morphology work; its lemma and morphology data are published under CC BY 4.0, while the Westminster Leningrad Codex text is public domain.
- **License:** Westminster Leningrad Codex text: public domain. Lemma/morphology data: CC BY 4.0.
- **How used in Bible O*:** Source provenance for lemma/morphology term modeling. Text NOT copied.
- **Source URL:** https://hb.openscriptures.org/
- **Usage boundary:** Reference model only. CC BY 4.0 attribution to "Open Scriptures Hebrew Bible Project" recorded here.

### 3. OpenBible.info Cross References
- **What it provides:** OpenBible.info Cross References describes about 340,000 cross-references identifying shared themes, words, events, or people.
- **License:** CC BY (Creative Commons Attribution License).
- **How used in Bible O*:** Model basis for bos:hasCrossReference.
- **Source URL:** https://www.openbible.info/labs/cross-references/
- **Usage boundary:** Relation model only. Cross-reference data treated as evidence links, not authoritative doctrine. No raw data ingested.

### 4. Composite Gospel Index RDF (Semantic Bible) — RESOLVED 2026-06-02

- **What it provides:** Composite Gospel Index RDF (CGI) is a public OWL/RDF representation of approximately 350 Gospel pericopes, authored by Sean Boisen. It models pericopes as discrete narrative units drawn from one or more of the four Gospel accounts (Matthew, Mark, Luke, John), with sequence properties (nextPericope, nextPericopeBySource) and numeric pericope identifiers.
- **License:** Creative Commons Attribution-NonCommercial-ShareAlike 2.0 (CC BY-NC-SA 2.0). Source: https://www.semanticbible.com/license.html — "All content on this website (including text, data files, and any original works), unless otherwise noted, is licensed under a Creative Commons License with conditions for attribution, non-commercial use, and distribution of derivative works." Copyright © 2003-2010 Sean Boisen.
- **CGI terms observed (not copied):** Pericope (class), PericopeSource (class), nextPericope (property), nextPericopeBySource (property), numeric pericope index (e.g. #235), Gospel author association, verse-count-per-source.
- **How used in Bible O*:** The pericope-as-unit structural pattern informed the independent composition of bos:Pericope, bos:GospelPassage, bos:ParallelPassage, bos:hasParallelPassage, and bos:pericopeIndex. No CGI RDF triples, identifiers, data records, or OWL axioms were copied or adapted verbatim. Terms are independently composed using OSIS reference conventions.
- **Directly modeled from CGI:** Nothing. All bos: terms are independently authored.
- **Independently modeled, CGI-informed:** bos:Pericope (subClassOf bos:Passage), bos:GospelPassage (subClassOf bos:Pericope), bos:ParallelPassage (subClassOf bos:Passage), bos:hasParallelPassage, bos:pericopeIndex. The concept of a pericope as a discrete, numbered Gospel narrative unit is the structural insight drawn from CGI.
- **Source URL:** https://semanticbible.com/cgi/cgi-in-rdf.html
- **License URL:** https://www.semanticbible.com/license.html
- **Attribution:** Composite Gospel Index RDF, Sean Boisen, SemanticBible.com, CC BY-NC-SA 2.0.
- **Usage boundary:** Structural pattern reference only (dcterms:source annotation in ontology). No CGI RDF data, no CGI triples, no CGI identifiers ingested. The Bible O* ontology is an independently authored work. The CC BY-NC-SA 2.0 license is compatible with non-commercial ontology publication; Bible O* is a non-commercial open ontology project.
- **PARTIAL resolution:** This entry was previously marked PARTIAL due to unclear license and unresolved term provenance. Resolved by direct fetch of https://www.semanticbible.com/license.html confirming CC BY-NC-SA 2.0 applies to all site content including data files, and by explicit enumeration of which terms are independently modeled vs. CGI-informed above.

### 5. W3C Public Standards
- RDF, RDFS, OWL, SHACL, PROV-O, DCTERMS, SKOS — all W3C public standards.
- License: W3C Software and Document License.
- All used as formal footing per standard practice.

## What Was NOT Used
- Lexham Bible Dictionary or Logos proprietary datasets.
- Any copyrighted Bible translation text.
- Any private namespace as foundation.
- Any gall: vocabulary.

## Audit Status
Source ledger composed: 2026-06-02
Audit updated: 2026-06-02 (CGI RDF PARTIAL resolved by Zadok / Water Gate agent)
Auditor: Meremoth / Old Gate; Zadok / Water Gate (CGI resolution)
Verification: Agent reports 1-4 confirm all sources are public-license clean.
CGI RDF PARTIAL: RESOLVED — CC BY-NC-SA 2.0 confirmed, terms independently modeled, dcterms:source annotations added to ontology.
