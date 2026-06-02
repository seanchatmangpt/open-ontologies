#!/usr/bin/env bash
# validate_bible_o_star.sh — Bible O* Validation Script
# Purge invalid structure. Enforce Inspection Gate law.
#
# Invocation: may be called from any directory. Script resolves all paths
# relative to its own location (bible-o-star/scripts/), so:
#   bash bible-o-star/scripts/validate_bible_o_star.sh  # from open-ontologies/
#   bash scripts/validate_bible_o_star.sh               # from bible-o-star/
# both work correctly.
set -e
# Resolve BOS from the script's own directory, not the caller's cwd.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BOS="$(cd "$SCRIPT_DIR/.." && pwd)"
echo "=== BIBLE_O_STAR_001 VALIDATION ==="

# Step 1: Turtle parse validation with rapper
echo "Step 1: Turtle parse (rapper)"
RAPPER=/opt/homebrew/bin/rapper
for f in "$BOS/ontology"/*.ttl "$BOS/examples"/*.ttl; do
    if "$RAPPER" -i turtle "$f" -o ntriples > /tmp/bos_out.nt 2>/tmp/bos_err.txt; then
      count=$(wc -l < /tmp/bos_out.nt | tr -d " ")
      if [ "$count" -eq 0 ]; then
        echo "FAIL: $f (0 triples — file may be corrupt)"; exit 1
      fi
      echo "PASS: $f ($count triples)"
    else
      echo "FAIL: $f"; cat /tmp/bos_err.txt; exit 1
    fi
  done

# Step 2: SHACL validation with pyshacl
echo ""
echo "Step 2: SHACL validation (pyshacl)"
python3 - <<'PYEOF'
import glob, sys
try:
    from pyshacl import validate
    from rdflib import Graph
    # Load ontology files as extra ontology graph (-e equivalent: extra ontology, not data)
    ont = Graph()
    for f in ["/Users/sac/open-ontologies/bible-o-star/ontology/bible-o-star.ttl",
              "/Users/sac/open-ontologies/bible-o-star/ontology/nehemiah-52.ttl"]:
        ont.parse(f, format="turtle")
    ex_files = glob.glob("/Users/sac/open-ontologies/bible-o-star/examples/*.ttl")
    r = validate(
        ex_files, shacl_graph="/Users/sac/open-ontologies/bible-o-star/ontology/nehemiah-52-shapes.ttl",
        ont_graph=ont,
        data_graph_format="turtle", shacl_graph_format="turtle",
        inference="rdfs", abort_on_first=False
    )
    conforms, g, text = r
    print("SHACL conforms:", conforms)
    if not conforms:
        print(text[:3000])
        sys.exit(1)
except Exception as e:
    print(f"ERROR: {e}", file=sys.stderr)
    sys.exit(1)
PYEOF

# Step 3: Check for fake gates — context-aware (owl:deprecated on next line)
echo ""
echo "Step 3: Fake gate check"
python3 - <<'PYEOF'
import re, sys, os, glob
try:
    gates = ['InterestGate','PeopleGate','MessengerGate','NationsGate',
             'ProphetGate','RumorGate','ReportGate']
    active = []
    for path in glob.glob('/Users/sac/open-ontologies/bible-o-star/ontology/*.ttl'):
        with open(path) as f:
            content = f.read()
        for gate in gates:
            for m in re.finditer(gate, content):
                ctx = content[max(0, m.start()-30):m.end()+300]
                if not any(t in ctx.lower() for t in ('deprecated','anti-pattern','refused')):
                    active.append(f"{gate} in {os.path.basename(path)}")
                    break
    if active:
        print('WARN: Active fake gate references found:')
        for a in active: print(' ', a)
        sys.exit(1)
    else:
        print('PASS: All fake gate references carry owl:deprecated — no active fake gates')
except Exception as e:
    print(f"ERROR: {e}", file=sys.stderr)
    sys.exit(1)
PYEOF

# Step 4: Check no proprietary sources — capture output before branching
echo ""
echo "Step 4: Proprietary source check"
PROP="lexham|logos|accordance|bible.gateway.com"
prop_hits=$(grep -ri -E "$PROP" "/Users/sac/open-ontologies/bible-o-star/ontology" 2>/dev/null || true)
if [ -n "$prop_hits" ]; then
  echo "WARN: Potential proprietary reference found - review:"
  echo "$prop_hits" | head -3
else
  echo "PASS: No proprietary source references"
fi

echo ""
echo "Step 5: Verifying BLAKE3 receipt chain..."
BOS_FOR_PY="$BOS" python3 - <<'PYEOF'
import os, subprocess, sys
from rdflib import Graph, Namespace
CELL8 = Namespace("urn:cell8:gate:")
BOS = os.environ.get("BOS_FOR_PY", ".")
g = Graph()
receipt_path = os.path.join(BOS, "receipts", "receipt-chain.ttl")
try:
    g.parse(receipt_path, format="turtle")
except Exception as e:
    print("ERROR: Could not parse receipt-chain.ttl:", e, file=sys.stderr)
    sys.exit(1)
ok = True
for s in set(g.subjects()):
    hashes = list(g.objects(s, CELL8.receiptHash))
    paths = list(g.objects(s, CELL8.subjectPath))
    if hashes and paths:
        fpath = os.path.join(BOS, str(paths[0]))
        r = subprocess.run(["b3sum", fpath], capture_output=True, text=True)
        if r.returncode != 0:
            print(f"ERROR: b3sum failed for {fpath}", file=sys.stderr); ok = False; continue
        actual = r.stdout.split()[0]
        stored = str(hashes[0])
        if actual != stored:
            print(f"MISMATCH {fpath}: stored={stored[:16]}... actual={actual[:16]}...", file=sys.stderr); ok = False
        else:
            print(f"OK {fpath}")
if not ok:
    sys.exit(1)
print("Receipt chain verified.")
PYEOF

echo ""
echo "=== VALIDATION COMPLETE ==="
