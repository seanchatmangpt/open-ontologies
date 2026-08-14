#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."
mkdir -p target/verifier

python3 -m py_compile tools/validate_dflss_dmedi.py
python3 tools/validate_dflss_dmedi.py --receipt target/verifier/dflss-dmedi-dod.json

cargo run --quiet -- ontology validate --input ontology/dflss-dmedi.ttl
cargo run --quiet -- ontology validate --input ontology/dflss-dmedi-shapes.ttl

python3 - <<'PY'
import json
from pathlib import Path

p = Path("target/verifier/dflss-dmedi-dod.json")
r = json.loads(p.read_text())
assert r["status"] == "ALIVE"
assert all(g["status"] == "PASSED" for g in r["gates"])
print(f"✓ DFLSS/DMEDI DoD ALIVE ({len(r['gates'])} gates); receipt={p}")
PY
