#!/usr/bin/env bash
# validate_bible_o_star.sh — Bible O* validation from any clean checkout.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BOS="$(cd "$SCRIPT_DIR/.." && pwd)"
RAPPER="${RAPPER:-$(command -v rapper || true)}"
B3SUM="${B3SUM:-$(command -v b3sum || true)}"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

if [[ -z "$RAPPER" ]]; then
  echo "ERROR: rapper is required (install raptor2-utils or set RAPPER)." >&2
  exit 1
fi
if [[ -z "$B3SUM" ]]; then
  echo "ERROR: b3sum is required (install b3sum or set B3SUM)." >&2
  exit 1
fi

export BOS_FOR_PY="$BOS"

echo "=== BIBLE O STAR VALIDATION ==="

echo "Step 1: Turtle parse"
shopt -s nullglob
files=("$BOS"/ontology/*.ttl "$BOS"/examples/*.ttl "$BOS"/governance/*.ttl "$BOS"/journal/*.ttl "$BOS"/versions/*.ttl)
if (( ${#files[@]} == 0 )); then
  echo "ERROR: no Turtle surfaces found under $BOS" >&2
  exit 1
fi
for file in "${files[@]}"; do
  out="$TMP_DIR/$(basename "$file").nt"
  err="$TMP_DIR/$(basename "$file").err"
  if "$RAPPER" -i turtle "$file" -o ntriples >"$out" 2>"$err"; then
    count="$(wc -l <"$out" | tr -d ' ')"
    if [[ "$count" -eq 0 ]]; then
      echo "FAIL: $file produced zero triples" >&2
      exit 1
    fi
    echo "PASS: ${file#"$BOS"/} ($count triples)"
  else
    echo "FAIL: $file" >&2
    cat "$err" >&2
    exit 1
  fi
done

echo
echo "Step 2: SHACL validation"
python3 - <<'PYEOF'
import glob
import os
import sys

from pyshacl import validate
from rdflib import Graph

bos = os.environ["BOS_FOR_PY"]
ontology = Graph()
for relative in ("ontology/bible-o-star.ttl", "ontology/nehemiah-52.ttl"):
    ontology.parse(os.path.join(bos, relative), format="turtle")

data = Graph()
for path in sorted(glob.glob(os.path.join(bos, "examples", "*.ttl"))):
    data.parse(path, format="turtle")

conforms, _, report = validate(
    data,
    shacl_graph=os.path.join(bos, "ontology", "nehemiah-52-shapes.ttl"),
    ont_graph=ontology,
    data_graph_format="turtle",
    shacl_graph_format="turtle",
    inference="rdfs",
    abort_on_first=False,
)
print("SHACL conforms:", conforms)
if not conforms:
    print(str(report)[:5000])
    sys.exit(1)
PYEOF

echo
echo "Step 3: Fake-gate refusal"
python3 - <<'PYEOF'
import glob
import os
import re
import sys

bos = os.environ["BOS_FOR_PY"]
forbidden = (
    "InterestGate",
    "PeopleGate",
    "MessengerGate",
    "NationsGate",
    "ProphetGate",
    "RumorGate",
    "ReportGate",
)
active = []
for path in sorted(glob.glob(os.path.join(bos, "ontology", "*.ttl"))):
    with open(path, encoding="utf-8") as stream:
        content = stream.read()
    for gate in forbidden:
        for match in re.finditer(gate, content):
            context = content[max(0, match.start() - 80): match.end() + 400].lower()
            if not any(token in context for token in ("deprecated", "anti-pattern", "refused")):
                active.append(f"{gate} in {os.path.basename(path)}")
                break
if active:
    print("ERROR: active fake-gate references:", file=sys.stderr)
    for finding in active:
        print(f"  {finding}", file=sys.stderr)
    sys.exit(1)
print("PASS: fake gates are absent or explicitly deprecated/refused")
PYEOF

echo
echo "Step 4: Proprietary-source refusal"
if grep -RInE 'lexham|logos|accordance|bible\.gateway\.com' "$BOS/ontology"; then
  echo "ERROR: proprietary source reference found" >&2
  exit 1
fi
echo "PASS: no proprietary source references"

echo
echo "Step 5: BLAKE3 receipt-chain verification"
B3SUM_FOR_PY="$B3SUM" python3 - <<'PYEOF'
import os
import subprocess
import sys

from rdflib import Graph, Namespace

bos = os.environ["BOS_FOR_PY"]
b3sum = os.environ["B3SUM_FOR_PY"]
cell8 = Namespace("urn:cell8:gate:")
graph = Graph()
graph.parse(os.path.join(bos, "receipts", "receipt-chain.ttl"), format="turtle")
verified = 0
for subject in sorted(set(graph.subjects()), key=str):
    hashes = list(graph.objects(subject, cell8.receiptHash))
    paths = list(graph.objects(subject, cell8.subjectPath))
    if not hashes or not paths:
        continue
    target = os.path.normpath(os.path.join(bos, str(paths[0])))
    if os.path.commonpath((bos, target)) != bos:
        print(f"ERROR: receipt path escapes Bible O Star boundary: {target}", file=sys.stderr)
        sys.exit(1)
    result = subprocess.run([b3sum, target], capture_output=True, text=True)
    if result.returncode != 0:
        print(f"ERROR: b3sum failed for {target}: {result.stderr}", file=sys.stderr)
        sys.exit(1)
    actual = result.stdout.split()[0]
    expected = str(hashes[0])
    if actual != expected:
        print(f"ERROR: hash mismatch for {target}: expected={expected} actual={actual}", file=sys.stderr)
        sys.exit(1)
    verified += 1
    print(f"PASS: {os.path.relpath(target, bos)}")
if verified == 0:
    print("ERROR: receipt chain contained no verifiable subject paths", file=sys.stderr)
    sys.exit(1)
print(f"Receipt chain verified ({verified} artifacts).")
PYEOF

echo
echo "=== VALIDATION COMPLETE ==="
