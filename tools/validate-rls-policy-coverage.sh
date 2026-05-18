#!/usr/bin/env bash
# validate-rls-policy-coverage.sh
#
# P0-B.7 hard gate: for every ggen:MaterializationKind with both
# ggen:emitsSql=true AND ggen:rlsRequired=true, at least one ODRL permission
# must target it (via ggen:rlsTargetKind in the RLS profile).
#
# Closes admission condition (3) of the Public Vocabulary Materialization
# Policy: no table emits without explicit ODRL coverage.

set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

python3 - <<'PYEOF'
import json
import subprocess
import sys
import glob

batch = [{"command": "clear"}]
for ttl in sorted(glob.glob("ontology/zoela/*.ttl")):
    batch.append({"command": "load", "args": [ttl]})
for ttl in sorted(glob.glob("ontology/profiles/*.ttl")):
    batch.append({"command": "load", "args": [ttl]})

sparql = """
PREFIX ggen: <https://open-ontologies.org/ggen#>
PREFIX odrl: <http://www.w3.org/ns/odrl/2/>
PREFIX skos: <http://www.w3.org/2004/02/skos/core#>
SELECT ?kind ?kindCode WHERE {
  ?kind a ggen:MaterializationKind ;
        ggen:emitsSql true ;
        ggen:rlsRequired true ;
        skos:notation ?kindCode .
  FILTER NOT EXISTS {
    ?action ggen:rlsTargetKind ?kind ;
            ggen:rlsOperation ?op .
    ?policy a odrl:Policy ;
            odrl:permission ?perm .
    ?perm odrl:action ?action .
  }
}
ORDER BY ?kindCode
"""
batch.append({"command": "query", "args": [sparql]})

proc = subprocess.run(
    ["onto", "batch", "-"],
    input=json.dumps(batch),
    capture_output=True,
    text=True,
)
if proc.returncode != 0:
    sys.stderr.write(f"onto batch failed (rc={proc.returncode}):\n{proc.stderr}\n")
    sys.exit(2)

uncovered = []
for line in proc.stdout.splitlines():
    obj = json.loads(line)
    if obj.get("command") == "query":
        res = obj.get("result", {})
        if "error" in res:
            sys.stderr.write(f"SPARQL error: {res['error']}\n")
            sys.exit(2)
        for b in res.get("results", []):
            iri = b.get("kind", "").strip('"<>')
            code = b.get("kindCode", "").strip('"').split('"')[0]
            uncovered.append((iri, code))

if uncovered:
    sys.stderr.write(
        "FAIL: RLS policy coverage — materialization kinds with emitsSql=true\n"
        "      and rlsRequired=true that lack ODRL permission coverage:\n\n"
    )
    for iri, code in uncovered:
        sys.stderr.write(f"  {code:<32} ({iri})\n")
    sys.stderr.write(
        "\nDoctrine: no Postgres RLS without explicit ODRL coverage.\n"
        "Fix one of:\n"
        "  (a) Add an ODRL permission in ontology/zoela/policy.ttl\n"
        "      whose action has ggen:rlsTargetKind pointing at this kind, OR\n"
        "  (b) Set ggen:emitsSql=false on the kind, OR\n"
        "  (c) Set ggen:rlsRequired=false (only if kind is explicitly public).\n"
        "(P0-B.7 — established 2026-05-18)\n"
    )
    sys.exit(1)

print("validate:rls-policy-coverage PASS — every SQL materialization kind has ODRL coverage")
PYEOF
