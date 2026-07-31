# BIBLE_O_STAR_001 — Gap Diagnostic Report

**Date:** 2026-06-02
**Assessor:** Gap Report Agent
**Scope:** Full diagnostic synthesis — why validate_bible_o_star.sh shows PASS but the ontology does not work

---

## Why validate_bible_o_star.sh Shows PASS but It Does Not Work

The script contains three independent failure modes, any one of which would cause a false PASS:

**Failure 1 — Corrupt data file silently passes the Turtle parse check.**
`ontology/nehemiah-52.ttl` contains a raw pyshacl text validation report, not Turtle RDF. The rapper
parse check uses `tail -1` and grep for the literal string "Error" or "error". Rapper's final output
line for a failed parse is `"rapper: Parsing returned 0 triples"` — which does not contain "Error".
The error lines appear earlier in the output but are thrown away by `tail -1`. Result: a completely
invalid file reports `PASS: ontology/nehemiah-52.ttl (0 triples)`.

**Failure 2 — SHACL validation targets an empty (corrupt) data graph.**
The script passes `nehemiah-52.ttl` as the SHACL data graph. Because that file is corrupt and contains
zero triples, pyshacl has nothing to validate. A data graph with zero instances trivially conforms to
every shape — there are no instances to violate anything. This is the vacuous-truth PASS: the receipt
claimed "Conforms: True" because there was no data, not because the data was valid.

**Failure 3 — SHACL is never run against the example files.**
All actual instance data lives in `examples/*.ttl`. The validation script never loads these files into
the data graph. When pyshacl is run against the full combined examples graph (458 triples), there are
21 SHACL violations — all in the `ClassConstraintComponent` for `bos:assignedToGate sh:class bos:Gate`.
The script would have to run: `pyshacl --data-graph examples/*.ttl --shacl-graph nehemiah-52-shapes.ttl`
to catch these. It does not.

**Additional: set -e does not save the script.**
The script declares `set -e` at the top. However, because the corrupt nehemiah-52.ttl parser error occurs
inside a subshell (`$(...)`) and the resulting line is assigned to a variable, bash does not propagate the
error exit. The rapper exit code 1 is captured but never checked; only the string content of stdout is
examined.

---

## Gap 1 (CRITICAL): nehemiah-52.ttl is corrupt — contains a pyshacl text report, not Turtle RDF

**What is wrong:**
`ontology/nehemiah-52.ttl` contains the text output of a previous pyshacl validation run, not valid Turtle
RDF. The file starts with `Validation Report` — not a Turtle prefix declaration. This means the central
domain ontology (which should define all 10 sanctioned gates, their subclasses, the fake gate registry,
and domain-specific classes like `bos:InspectionGate`, `bos:MusterRegistry`, `bos:UsuryAudit`) is entirely
absent from disk.

**Evidence:**
```
$ head -3 ontology/nehemiah-52.ttl
Validation Report
Conforms: False
Results (2):
```
```
$ rapper -i turtle ontology/nehemiah-52.ttl -o ntriples 2>&1
rapper: Error - ...nehemiah-52.ttl:0 - syntax error at 'V'
rapper: Parsing returned 0 triples
```
BLAKE3 hash of current file: `e21876b2c1d1c9caf8cfb5d186a110969cfb51f44171bedf27e93e1cd64fed09`
BLAKE3 hash from SNAPSHOT_002 manifest: `f05dd51621ad4364fced49db60a9a0284d5aaf855a9f3b16f863f4af0d256a5d`
Hash mismatch confirms the file was overwritten after the receipt was created.

**Fix:**
The file must be reconstructed from the content it is documented to have contained. Based on the
ADVERSARIAL_REVIEW_002 findings and the SHACL shapes file, `nehemiah-52.ttl` must declare:
1. All 10 sanctioned gate individuals typed as `bos:Gate` and `owl:NamedIndividual`
2. The fake gate anti-pattern registry (7 refused gates with `owl:deprecated true`)
3. Domain-specific classes: `bos:Builder`, `bos:WallSection`, `bos:Gate`, `bos:InspectionGate` (subClassOf Gate),
   `bos:CourierRecord`, `bos:FalseReport`, `bos:MockerFeedback`, `bos:UsuryLedgerRecord`,
   `bos:MusterLedgerRecord`, `bos:PropheticProclamation`, `bos:NationsLedgerRecord`,
   `bos:InspectionReceipt`, `bos:Verdict`, `bos:MusterRegistry`, `bos:UsuryAudit`
4. Domain-specific properties: `bos:assignedToGate`, `bos:routesToGate`, `bos:hasVerdict`,
   `bos:hasReceipt`, `bos:hasBuilder`, `bos:hasWallSection`, `bos:buildsWallSection`,
   `bos:hasTimestamp`, `bos:hasMusterRecord`, `bos:hasUsuryAudit`, `bos:refusesPoison`
5. The 3 verdict individuals: `bos:VerdictAlive`, `bos:VerdictPartial`, `bos:VerdictBlocked`

**File to change:** `/Users/sac/open-ontologies/bible-o-star/ontology/nehemiah-52.ttl`
**Action:** Replace entire file content with valid Turtle RDF containing the above declarations.

---

## Gap 2 (CRITICAL): validate_bible_o_star.sh Step 1 — rapper exit code not checked; error in stderr silently discarded

**What is wrong:**
The script captures rapper output as `result=$("$RAPPER" -i turtle "$f" -o ntriples 2>&1 | tail -1)` and
then checks `if echo "$result" | grep -q "Error\|error"`. For a corrupt TTL file, rapper writes its error
messages to stderr (which is merged via `2>&1`), but the error lines appear BEFORE the final line. The
`tail -1` discards all error lines, retaining only `"rapper: Parsing returned 0 triples"` — which does
not match the grep pattern. The corrupt file is declared PASS.

**Evidence:**
Full rapper output for corrupt nehemiah-52.ttl:
```
rapper: Parsing URI file:///...nehemiah-52.ttl with parser turtle
rapper: Serializing with serializer ntriples
rapper: Error - ...nehemiah-52.ttl:0 - syntax error at 'V'
rapper: Failed to parse file .../nehemiah-52.ttl turtle content
rapper: Parsing returned 0 triples
```
`tail -1` keeps only: `rapper: Parsing returned 0 triples` — no "Error" string found, so grep returns exit 1
(no match), the `if` condition is false, and the script prints `PASS`.

**Fix:**
Replace the grep-on-tail approach with a direct rapper exit code check AND a triple count check:
```bash
for f in "$BOS/ontology"/*.ttl "$BOS/examples"/*.ttl; do
  if "$RAPPER" -i turtle "$f" -o ntriples > /tmp/rapper_out.nt 2>/tmp/rapper_err.txt; then
    triples=$(wc -l < /tmp/rapper_out.nt)
    if [ "$triples" -eq 0 ]; then
      echo "WARN: $f parsed but produced 0 triples — may be empty or corrupt"
    else
      echo "PASS: $f ($triples triples)"
    fi
  else
    echo "FAIL: $f"; cat /tmp/rapper_err.txt; exit 1
  fi
done
```

**File to change:** `/Users/sac/open-ontologies/bible-o-star/scripts/validate_bible_o_star.sh`

---

## Gap 3 (CRITICAL): SHACL validation targets an empty data graph — 21 real violations go undetected

**What is wrong:**
Step 2 of the validation script passes `nehemiah-52.ttl` (which is currently corrupt, yielding 0 triples)
as the pyshacl data graph. Even if `nehemiah-52.ttl` were valid, it is an ontology file with no instance
data — it contains class declarations, not individuals. The shaped classes (Builder, WallSection,
InspectionReceipt, etc.) have no instances in that file. pyshacl correctly reports `Conforms: True`
because there is nothing to violate.

When SHACL is run against the actual instance data (`examples/*.ttl` combined, 458 triples), there are
**21 ClassConstraintComponent violations** — every gate in the dataset is referenced via `bos:assignedToGate`
but is not typed `a bos:Gate` in the same graph. The `bos:BuilderShape` and `bos:WallSectionShape` shapes
both require `bos:assignedToGate sh:class bos:Gate`, which fails for all 21 focal nodes.

**Evidence:**
Running pyshacl against combined examples returns `Conforms: False` with 21 violations:
```
Focus Node: bos:DungGateSection — Value bos:DungGate does not have class bos:Gate
Focus Node: bos:MalkijahSonOfRechab — Value bos:DungGate does not have class bos:Gate
Focus Node: bos:EastGateSection — Value bos:EastGate does not have class bos:Gate
... (18 more violations, all same pattern)
```
Files missing gate type assertion (`a bos:Gate`):
- `examples/dung-gate-record.ttl` — missing `bos:DungGate a bos:Gate`
- `examples/east-gate-record.ttl` — missing `bos:EastGate a bos:Gate`
- `examples/fountain-gate-record.ttl` — missing `bos:FountainGate a bos:Gate`
- `examples/horse-gate-record.ttl` — missing `bos:HorseGate a bos:Gate`
- `examples/muster-ledger-record.ttl` — missing `bos:FishGate a bos:Gate`, `bos:SheepGate a bos:Gate`
- `examples/old-gate-record.ttl` — missing `bos:OldGate a bos:Gate`
- `examples/sheep-gate-record.ttl` — missing `bos:SheepGate a bos:Gate`
- `examples/usury-ledger-record.ttl` — missing `bos:DungGate a bos:Gate`
- `examples/valley-gate-record.ttl` — missing `bos:ValleyGate a bos:Gate`
- `examples/water-gate-pericope.ttl` — `bos:WaterGate a owl:NamedIndividual` but missing `a bos:Gate`
- `examples/water-gate-record.ttl` — missing `bos:WaterGate a bos:Gate`

**Fix — Part A: Add gate type assertion to each affected example file.**
Each file must add a stanza like:
```turtle
bos:DungGate a bos:Gate ;
    rdfs:label "Dung Gate" ;
    bos:hasCanonicalReference "Neh.3.13-14"^^xsd:string .
```
(using the correct canonical reference per gate)

**Fix — Part B: Update the validation script to test the full data graph.**
Replace the Step 2 pyshacl call:
```python
# OLD — validates empty ontology graph
data_graph='/Users/sac/open-ontologies/bible-o-star/ontology/nehemiah-52.ttl'

# NEW — validates all instance data
import glob
from rdflib import Dataset
ds = Dataset()
for f in glob.glob('/Users/sac/open-ontologies/bible-o-star/examples/*.ttl'):
    ds.parse(f, format='turtle')
r = validate(
    data_graph=ds,
    shacl_graph='/Users/sac/open-ontologies/bible-o-star/ontology/nehemiah-52-shapes.ttl',
    shacl_graph_format='turtle',
    inference='rdfs',
    abort_on_first=False
)
```

**File to change:** Every `examples/*.ttl` file listed above, plus `scripts/validate_bible_o_star.sh`

---

## Gap 4 (CRITICAL): Receipt chain hashes are stale — bible-o-star.ttl and nehemiah-52.ttl hashes do not match disk

**What is wrong:**
`receipts/receipt-chain.ttl` records BLAKE3 hashes for the four ontology files. Two of the four hashes
do not match the current files on disk. This means the receipt chain does not cover the current state of
the ontology. Any verifier checking the chain will find it invalid.

**Evidence:**
```
ontology/bible-o-star.ttl
  recorded: 0a4211035f19a4415403d56d9099877b7fce0e817c0da5511696256ed40ef04d
  actual:   f7f43fb56c9e8f8ae36c22aba7a1006c5c6708a832fa119c187dbb62fb229216
  status:   MISMATCH

ontology/nehemiah-52.ttl
  recorded: f05dd51621ad4364fced49db60a9a0284d5aaf855a9f3b16f863f4af0d256a5d
  actual:   e21876b2c1d1c9caf8cfb5d186a110969cfb51f44171bedf27e93e1cd64fed09
  status:   MISMATCH

ontology/nehemiah-52-shapes.ttl
  recorded: f54ff8982fb817a4d3e174af23e33bdb95de2d779920ff99c590e325dfb44785
  actual:   f54ff8982fb817a4d3e174af23e33bdb95de2d779920ff99c590e325dfb44785
  status:   MATCH

ontology/source-ledger.ttl
  recorded: 37de03b9299a7dd6910213b5ab9e05bd9a0237504f477a4ea8b689c1aaa9700b
  actual:   37de03b9299a7dd6910213b5ab9e05bd9a0237504f477a4ea8b689c1aaa9700b
  status:   MATCH
```

The `nehemiah-52.ttl` hash mismatch is the fingerprint of the file corruption event: the snapshot recorded
hash `f05dd5...` when the file was valid Turtle; the current hash `e21876...` corresponds to the corrupt
validation-report content.

**Fix:**
1. Restore `nehemiah-52.ttl` to valid Turtle content (see Gap 1).
2. Recompute the BLAKE3 hash of the fixed `nehemiah-52.ttl`.
3. Recompute the BLAKE3 hash of `bible-o-star.ttl` (which was also modified after the chain was written).
4. Update `receipts/receipt-chain.ttl` with the new hashes and a new `dcterms:modified` timestamp.
5. Re-sign the updated chain with Ed25519 (`bos:SealAssertion`).

**File to change:** `/Users/sac/open-ontologies/bible-o-star/receipts/receipt-chain.ttl`
**Command to compute hashes:**
```bash
b3sum ontology/bible-o-star.ttl ontology/nehemiah-52.ttl ontology/nehemiah-52-shapes.ttl ontology/source-ledger.ttl
```

---

## Gap 5 (MAJOR): bos:MusterRegistry and bos:UsuryAudit are used as RDF types but never declared as owl:Class

**What is wrong:**
`examples/muster-ledger-record.ttl` contains:
```turtle
bos:MusterRegistry001 a bos:MusterRegistry ;
```
`examples/usury-ledger-record.ttl` contains:
```turtle
bos:UsuryAudit001 a bos:UsuryAudit ;
```
Neither `bos:MusterRegistry` nor `bos:UsuryAudit` is declared as `a owl:Class` in any ontology file.
A reasoner treats these as `owl:Thing` with no constraints, silently accepting any instance. SPARQL
queries against `?x a bos:MusterRegistry` will return no results unless the class is declared.

**Evidence:**
```bash
$ grep -r "MusterRegistry\|UsuryAudit" ontology/*.ttl
(no output)
```
Both class names appear only in `examples/*.ttl`, never in any `ontology/*.ttl` file.

**Fix:**
Add to `ontology/nehemiah-52.ttl` (which must be reconstructed per Gap 1):
```turtle
bos:MusterRegistry a owl:Class ;
    rdfs:label "Muster Registry" ;
    rdfs:comment "The accountability registry of all wall-section builders and their assignments. Source: Nehemiah 3." .

bos:UsuryAudit a owl:Class ;
    rdfs:label "Usury Audit" ;
    rdfs:comment "A documented audit of economic extraction practices — debts, pledges, and their resolution. Source: Nehemiah 5." .
```

**File to change:** `/Users/sac/open-ontologies/bible-o-star/ontology/nehemiah-52.ttl`

---

## Gap 6 (MAJOR): bos:hasTimestamp is used in examples but never declared in any ontology file

**What is wrong:**
`examples/inspection-gate-receipt.ttl` line 48 uses:
```turtle
bos:hasTimestamp "445-09-25"^^xsd:string ;
```
`bos:hasTimestamp` is not declared as `a owl:DatatypeProperty` in any ontology file. Any application
relying on this property for temporal ordering will find zero results when querying by `bos:hasTimestamp`.

**Evidence:**
```bash
$ grep -r "hasTimestamp" ontology/*.ttl
(no output)
```

**Fix:**
Add to `ontology/nehemiah-52.ttl`:
```turtle
bos:hasTimestamp a owl:DatatypeProperty ;
    rdfs:label "has timestamp" ;
    rdfs:range xsd:string ;
    rdfs:comment "A string-form timestamp for this receipt or record. For machine-readable temporal ordering, prefer dcterms:date with xsd:date datatype." .
```

**File to change:** `/Users/sac/open-ontologies/bible-o-star/ontology/nehemiah-52.ttl`

---

## Gap 7 (MAJOR): bos:InspectionGate used simultaneously as owl:Class IRI and as an individual IRI — OWL Full violation

**What is wrong:**
Per ADVERSARIAL_REVIEW_002 finding Mi1: `bos:InspectionGate` is declared as an `owl:Class` in
`nehemiah-52.ttl` AND referenced as a named individual (`bos:InspectionGate a bos:Gate`) in
`examples/inspection-gate-receipt.ttl` and `examples/courier-false-report-record.ttl`. In OWL DL,
a class IRI and an individual IRI must be disjoint. Using the same IRI for both forces the ontology
into OWL Full, which is formally undecidable.

**Evidence:**
In `examples/inspection-gate-receipt.ttl`:
```turtle
bos:InspectionGate a bos:Gate ;
    rdfs:label "Inspection Gate" ;
    bos:hasCanonicalReference "Neh.3.31"^^xsd:string .
```
The individual `bos:InspectionGate` shares its IRI with the class `bos:InspectionGate` declared in
the ontology.

**Fix:**
Option A (preferred): Rename the individual to `bos:TheInspectionGate` in all example files:
```turtle
bos:TheInspectionGate a bos:Gate, owl:NamedIndividual ;
    rdfs:label "Inspection Gate" ;
    bos:hasCanonicalReference "Neh.3.31"^^xsd:string .
```
Then update all references from `bos:InspectionGate` (as individual) to `bos:TheInspectionGate` in:
- `examples/inspection-gate-receipt.ttl`
- `examples/courier-false-report-record.ttl`
- `examples/mocker-feedback-record.ttl`

**File to change:** `examples/inspection-gate-receipt.ttl`, `examples/courier-false-report-record.ttl`,
`examples/mocker-feedback-record.ttl`

---

## Gap 8 (MAJOR): Validation script crashes on Step 2 with unhandled Python exception — no graceful error reporting

**What is wrong:**
The validation script's Step 2 invokes `python3 -c "..."` which raises an unhandled exception when
`nehemiah-52.ttl` cannot be parsed as Turtle. The Python traceback is printed to stdout/stderr but the
script does not intercept it. The `set -e` directive causes the script to abort at this point, but the
user reported seeing PASS — meaning either a previous version of the script (without the corrupt file)
was used to generate the receipt, or the script was run with `set +e` / in a context where exit codes
were suppressed.

The result is that Step 3 (fake gate check) and Step 4 (proprietary source check) are never executed
when Step 2 crashes.

**Evidence:**
```
$ bash scripts/validate_bible_o_star.sh 2>&1 | tail -5
...rdflib.plugins.parsers.notation3.BadSyntax: <no detail available>
Script exit code: 1
```
The script exits at Step 2. Step 3 and Step 4 results are never produced.

**Fix:**
Wrap the Step 2 Python call in explicit error handling:
```python
try:
    r = validate(...)
    conforms, graph, text = r
    print('SHACL conforms:', conforms)
    if not conforms:
        print(text)
        sys.exit(1)
except Exception as e:
    print(f'FAIL: SHACL validation error: {e}')
    sys.exit(1)
```
And fix the data-graph target to use examples (see Gap 3).

**File to change:** `/Users/sac/open-ontologies/bible-o-star/scripts/validate_bible_o_star.sh`

---

## Gap 9 (MINOR): water-gate-pericope.ttl types bos:WaterGate as owl:NamedIndividual only — missing bos:Gate type

**What is wrong:**
`examples/water-gate-pericope.ttl` declares:
```turtle
bos:WaterGate a owl:NamedIndividual ;
```
All other files that declare gate individuals use `a bos:Gate`. This causes a SHACL violation because
`bos:WaterGate` does not satisfy `sh:class bos:Gate` for the `bos:assignedToGate` property shape.

**Evidence:**
SHACL violation: `Focus Node: bos:WaterGateSection — Value bos:WaterGate does not have class bos:Gate`

**Fix:**
In `examples/water-gate-pericope.ttl`, change:
```turtle
bos:WaterGate a owl:NamedIndividual ;
```
to:
```turtle
bos:WaterGate a bos:Gate, owl:NamedIndividual ;
```

**File to change:** `/Users/sac/open-ontologies/bible-o-star/examples/water-gate-pericope.ttl`

---

## Gap 10 (MINOR): SNAPSHOT_002 hash for nehemiah-52.ttl is stale and documents the pre-corruption state

**What is wrong:**
`versions/SNAPSHOT_002.md` records hash `f05dd51621ad4364fced49db60a9a0284d5aaf855a9f3b16f863f4af0d256a5d`
for `nehemiah-52.ttl`. This was the hash when the file was valid Turtle. After reconstruction, a new
snapshot must be emitted to document the repaired state. The rollback procedure in the snapshot currently
points to a version of the file that does not exist in any recoverable location.

**Evidence:**
```
versions/SNAPSHOT_002.md: f05dd51621... ./ontology/nehemiah-52.ttl
actual disk hash:          e21876b2c1... (corrupt content)
```

**Fix:**
After Gap 1 is resolved (nehemiah-52.ttl reconstructed and verified), emit SNAPSHOT_003:
```bash
b3sum ontology/*.ttl examples/*.ttl governance/*.ttl journal/*.ttl > versions/SNAPSHOT_003.md
```

**File to change:** `versions/` — create `SNAPSHOT_003.md` and `snapshot-003.ttl`

---

## Summary

| Gap | Title | Severity | Root Cause | Status |
|-----|-------|----------|------------|--------|
| 1 | nehemiah-52.ttl corrupt — contains pyshacl report, not Turtle | CRITICAL | File overwritten by a prior agent pyshacl run | OPEN |
| 2 | validate_bible_o_star.sh Step 1 — rapper exit code not checked | CRITICAL | `tail -1` discards error lines; grep never sees "Error" | OPEN |
| 3 | SHACL targets empty graph — 21 real violations hidden | CRITICAL | Script passes ontology TTL (0 instances) as data graph | OPEN |
| 4 | Receipt chain hashes stale — 2 of 4 files do not match | CRITICAL | Files modified after receipt chain written | OPEN |
| 5 | bos:MusterRegistry and bos:UsuryAudit undeclared as owl:Class | MAJOR | Classes used in examples but never declared in ontology | OPEN |
| 6 | bos:hasTimestamp undeclared property | MAJOR | Property used in examples but never declared in ontology | OPEN |
| 7 | bos:InspectionGate class/individual IRI collision — OWL Full | MAJOR | Same IRI used for class and named individual | OPEN |
| 8 | Validation script crashes on Step 2 with unhandled exception | MAJOR | No try/except around pyshacl call | OPEN |
| 9 | water-gate-pericope.ttl types WaterGate as NamedIndividual only | MINOR | Missing `a bos:Gate` type assertion | OPEN |
| 10 | SNAPSHOT_002 hash for nehemiah-52.ttl documents pre-corruption state | MINOR | Snapshot not updated after file was corrupted | OPEN |

---

## Exact Commands to Verify Each Fix

### Gap 1 — Verify nehemiah-52.ttl is valid Turtle after reconstruction
```bash
/opt/homebrew/bin/rapper -i turtle /Users/sac/open-ontologies/bible-o-star/ontology/nehemiah-52.ttl -o ntriples 2>&1
# Expected: last line shows "> 0 triples" with no error lines
python3 -c "
from rdflib import Graph
g = Graph()
g.parse('/Users/sac/open-ontologies/bible-o-star/ontology/nehemiah-52.ttl', format='turtle')
print(f'Triples parsed: {len(g)}')
assert len(g) > 0, 'FAIL: zero triples'
print('PASS')
"
```

### Gap 2 — Verify rapper exit code detection works
```bash
# Test with a valid file (should exit 0)
/opt/homebrew/bin/rapper -i turtle /Users/sac/open-ontologies/bible-o-star/ontology/bible-o-star.ttl -o ntriples > /dev/null 2>&1
echo "Exit: $?"  # Expected: 0

# Test with a corrupt file (should exit 1)
/opt/homebrew/bin/rapper -i turtle /Users/sac/open-ontologies/bible-o-star/ontology/nehemiah-52.ttl -o ntriples > /dev/null 2>&1
echo "Exit: $?"  # Expected: 1 (after corruption is present) or 0 after fix
```

### Gap 3 — Verify SHACL passes against full examples graph
```bash
cd /Users/sac/open-ontologies/bible-o-star && python3 -c "
import glob
from pyshacl import validate
from rdflib import Dataset
ds = Dataset()
for f in glob.glob('examples/*.ttl'):
    ds.parse(f, format='turtle')
conforms, g, text = validate(
    data_graph=ds,
    shacl_graph='ontology/nehemiah-52-shapes.ttl',
    shacl_graph_format='turtle',
    inference='rdfs',
    abort_on_first=False
)
print('SHACL conforms:', conforms)
if not conforms:
    print(text)
"
# Expected: SHACL conforms: True
```

### Gap 4 — Verify receipt chain hashes match current files
```bash
cd /Users/sac/open-ontologies/bible-o-star
b3sum ontology/bible-o-star.ttl ontology/nehemiah-52.ttl ontology/nehemiah-52-shapes.ttl ontology/source-ledger.ttl
# Compare output against cell8:receiptHash values in receipts/receipt-chain.ttl
# All 4 must match
```

### Gap 5 — Verify undeclared classes are now declared
```bash
grep -c "bos:MusterRegistry a owl:Class\|bos:UsuryAudit a owl:Class" /Users/sac/open-ontologies/bible-o-star/ontology/nehemiah-52.ttl
# Expected: 2
```

### Gap 6 — Verify bos:hasTimestamp is declared
```bash
grep "hasTimestamp" /Users/sac/open-ontologies/bible-o-star/ontology/nehemiah-52.ttl
# Expected: one or more lines declaring it as owl:DatatypeProperty
```

### Gap 7 — Verify no class/individual IRI collision
```bash
python3 -c "
from rdflib import Graph, Namespace, RDF, OWL
g = Graph()
import glob
for f in glob.glob('/Users/sac/open-ontologies/bible-o-star/ontology/*.ttl'):
    try: g.parse(f, format='turtle')
    except: pass
for f in glob.glob('/Users/sac/open-ontologies/bible-o-star/examples/*.ttl'):
    try: g.parse(f, format='turtle')
    except: pass
BOS = Namespace('https://open-ontologies.org/bible-o-star#')
classes = set(g.subjects(RDF.type, OWL.Class))
individuals = set(g.subjects(RDF.type, BOS.Gate))
collision = classes & individuals
if collision:
    print('COLLISION:', collision)
else:
    print('PASS: no class/individual IRI collision')
"
```

### Gap 8 — Verify script runs to completion without crash
```bash
bash /Users/sac/open-ontologies/bible-o-star/scripts/validate_bible_o_star.sh 2>&1
echo "Exit: $?"
# Expected: exit code 0 and "=== VALIDATION COMPLETE ===" at end
```

### Gap 9 — Verify WaterGate has bos:Gate type
```bash
grep "bos:WaterGate a" /Users/sac/open-ontologies/bible-o-star/examples/water-gate-pericope.ttl
# Expected: bos:WaterGate a bos:Gate, owl:NamedIndividual
```

### Gap 10 — Verify new snapshot hashes match files after all fixes applied
```bash
cd /Users/sac/open-ontologies/bible-o-star
b3sum ontology/*.ttl examples/*.ttl | sort > /tmp/current_hashes.txt
diff /tmp/current_hashes.txt versions/SNAPSHOT_003.md  # after snapshot-003 is created
# Expected: no diff
```
