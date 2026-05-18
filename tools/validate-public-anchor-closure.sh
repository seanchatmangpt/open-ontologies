#!/usr/bin/env bash
# validate-public-anchor-closure.sh
#
# P0-B.5 hard gate: enforces the Public Vocabulary Materialization Policy.
#
# Every owl:Class declared under the ZOE namespace (https://zoela.org/ontology/)
# must reach at least one public-vocabulary anchor class via rdfs:subClassOf+.
# Public vocab is anything OUTSIDE the ZOE namespace and not owl:Thing.
#
# Why: under the Public Vocabulary Materialization Policy, ZOE classes are
# local specializations of public interop roles (schema:Person, schema:Action,
# prov:Activity, odrl:Policy, etc.). A class with no public anchor cannot be
# materialized — it has no interop contract to project from.
#
# Doctrine reference:
#   ~/.claude/projects/-Users-sac-open-ontologies/memory/feedback_public_vocab_materialization.md

set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

# Use onto batch to load all ontology/zoela/*.ttl and run the closure SPARQL.
# Result is NDJSON; one of the lines is the query result.

python3 - <<'PYEOF'
import json
import subprocess
import sys
import glob

batch = [{"command": "clear"}]
for ttl in sorted(glob.glob("ontology/zoela/*.ttl")):
    batch.append({"command": "load", "args": [ttl]})

sparql = """
PREFIX zoe: <https://zoela.org/ontology/>
PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?class WHERE {
  ?class a owl:Class .
  FILTER(STRSTARTS(STR(?class), "https://zoela.org/ontology/"))
  FILTER NOT EXISTS {
    ?class rdfs:subClassOf+ ?anchor .
    FILTER(!STRSTARTS(STR(?anchor), "https://zoela.org/ontology/"))
    FILTER(?anchor != owl:Thing)
    FILTER(isIRI(?anchor))
  }
}
ORDER BY ?class
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

unanchored = []
for line in proc.stdout.splitlines():
    obj = json.loads(line)
    if obj.get("command") == "query":
        res = obj.get("result", {})
        if "error" in res:
            sys.stderr.write(f"SPARQL error: {res['error']}\n")
            sys.exit(2)
        for b in res.get("results", []):
            iri = b.get("class", "").strip('"').replace("<", "").replace(">", "")
            unanchored.append(iri)

if unanchored:
    sys.stderr.write(
        "FAIL: public-anchor closure violation — ZOE classes lack rdfs:subClassOf+\n"
        "      to any public-vocabulary anchor.\n\n"
    )
    for u in unanchored:
        sys.stderr.write(f"  {u}\n")
    sys.stderr.write(
        "\nDoctrine: every materializable ZOE class must specialize at least one\n"
        "  public interop role (schema:*, foaf:*, sioc:*, org:*, as:*, prov:*,\n"
        "  odrl:*, time:*, skos:*, dcterms:*).\n"
        "Fix: add rdfs:subClassOf <public-anchor-iri> to each listed class.\n"
        "(P0-B.5 — established 2026-05-18)\n"
    )
    sys.exit(1)

print("validate:public-anchor-closure PASS — all ZOE owl:Class reach a public anchor")
PYEOF
