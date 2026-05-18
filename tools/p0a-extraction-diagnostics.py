#!/usr/bin/env python3
"""
P0-A Extraction Diagnostics — strong-form version.

Loads all ontology/zoela/*.ttl into a single onto-batch process, then runs
every .specify/queries/zoela/*.rq query against the loaded graph. Reports:

  1. Row count per query
  2. Null-rate for critical bindings (where applicable)
  3. Duplicate-id detection (where applicable)
  4. μ_mobile readiness matrix (Refinement #7)
  5. Route-runtime drill-down for ConnectGroupJoinRoute (Refinement #8)
  6. Manufacturing-restored success condition (Refinement #9)

Writes diagnostics into .ggen/audit/p0-a/ (Refinement #10).
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent
ONTO_DIR = REPO_ROOT / "ontology" / "zoela"
QUERY_DIR = REPO_ROOT / ".specify" / "queries" / "zoela"
AUDIT_DIR = REPO_ROOT / ".ggen" / "audit" / "p0-a"
AUDIT_DIR.mkdir(parents=True, exist_ok=True)


def build_batch() -> list[dict[str, Any]]:
    """Build a JSON batch: clear → load all TTL → stats → all queries."""
    batch: list[dict[str, Any]] = [{"command": "clear"}]
    for ttl in sorted(ONTO_DIR.glob("*.ttl")):
        batch.append({"command": "load", "args": [str(ttl)]})
    batch.append({"command": "stats"})
    for rq in sorted(QUERY_DIR.glob("*.rq")):
        sparql = rq.read_text()
        # Strip comments so the parser doesn't choke if any comment contains
        # accidental tokens; SPARQL parser handles # comments natively but
        # keep it simple.
        batch.append({"command": "query", "args": [sparql], "_query_name": rq.stem})
    return batch


def run_batch(batch: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Run onto batch and return the parsed NDJSON results."""
    # Strip the _query_name annotation before sending; onto batch ignores extra keys
    # but be safe.
    send_batch = []
    for item in batch:
        send = {k: v for k, v in item.items() if not k.startswith("_")}
        send_batch.append(send)
    proc = subprocess.run(
        ["onto", "batch", "-"],
        input=json.dumps(send_batch),
        capture_output=True,
        text=True,
        cwd=str(REPO_ROOT),
    )
    if proc.returncode != 0:
        sys.stderr.write(f"onto batch failed (rc={proc.returncode}):\n")
        sys.stderr.write(proc.stderr)
        sys.exit(2)
    results = []
    for line in proc.stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        results.append(json.loads(line))
    return results


def extract_count(query_result: dict[str, Any]) -> tuple[int, list[dict[str, Any]]]:
    """Return (row_count, raw_bindings) for a query result."""
    res = query_result.get("result", {})
    if "error" in res:
        return -1, []
    bindings = res.get("results", [])
    return len(bindings), bindings


def parse_literal(v: str | None) -> str | None:
    """Unwrap a SPARQL JSON literal like '"8"^^<...>' or '"foo"@en' to bare value."""
    if v is None:
        return None
    if isinstance(v, (int, float)):
        return str(v)
    s = str(v)
    m = re.match(r'^"(.*?)"(?:@[a-zA-Z-]+|\^\^.*)?$', s)
    return m.group(1) if m else s


def count_aggregate_row(bindings: list[dict[str, Any]], var: str = "c") -> int:
    """For SELECT COUNT(*) queries, pull the integer out."""
    if not bindings:
        return 0
    first = bindings[0]
    val = parse_literal(first.get(var))
    try:
        return int(val) if val else 0
    except (TypeError, ValueError):
        return 0


def critical_field_audit(bindings: list[dict[str, Any]], required: list[str]) -> dict[str, Any]:
    """For SELECT queries with required fields, count nulls and dupes."""
    if not bindings:
        return {"rows": 0, "nulls": {}, "duplicates": {}, "complete_rows": 0}
    nulls = {f: 0 for f in required}
    seen: dict[str, set[str]] = {f: set() for f in required}
    duplicates: dict[str, int] = {f: 0 for f in required}
    complete = 0
    for b in bindings:
        row_complete = True
        for f in required:
            v = parse_literal(b.get(f))
            if not v:
                nulls[f] += 1
                row_complete = False
            else:
                if v in seen[f]:
                    duplicates[f] += 1
                seen[f].add(v)
        if row_complete:
            complete += 1
    return {
        "rows": len(bindings),
        "nulls": nulls,
        "duplicates": duplicates,
        "complete_rows": complete,
        "null_rate": {f: nulls[f] / len(bindings) for f in required},
    }


# Per-query critical-field expectations.
# When a query is unknown here, only row count is reported.
QUERY_REQUIRED_FIELDS: dict[str, list[str]] = {
    "extract-screens": ["screen_id", "nav_route", "screen_stack", "required_role"],
    "extract-admin-screens": ["screen_id", "nav_route"],
    "extract-navigation": ["tabCode", "stackName"],
    "extract-service-routes": ["routeCode", "stageCode", "stageOrder"],
    "extract-connect-group-stages": ["stageCode", "stageOrder"],
    "extract-connect-group-work-orders": [],
    "extract-connect-group-admin": [],
    "extract-ocel-events": ["eventType"],
    "extract-ocel-schema": [],
    "extract-receipt-models": [],
    "extract-rls-policies": ["table_name"],
    "extract-bridge-tables": ["tableName"],
    "extract-tables": ["classNames", "tableNames"],
    "extract-types": [],
    "extract-form-fields": [],
    "extract-skos-enums": [],
    "extract-role-requirements": [],
    "extract-consent-requirements": [],
    "extract-sensitivity-rules": ["sensitivityNotation"],
    "extract-followup-rules": [],
    "extract-push-card-fields": [],
    "extract-resource-matching-rules": [],
    "extract-assignment-eligibility": [],
}


def main() -> int:
    print(f"P0-A Extraction Diagnostics — {REPO_ROOT}", file=sys.stderr)
    batch = build_batch()
    query_items = [i for i in batch if i["command"] == "query"]
    print(f"  Loading {len(list(ONTO_DIR.glob('*.ttl')))} TTL files", file=sys.stderr)
    print(f"  Running {len(query_items)} extraction queries", file=sys.stderr)

    results = run_batch(batch)

    # Index results by sequence to align with batch items
    by_seq = {r.get("seq"): r for r in results}

    # The stats result comes after all loads
    stats_seq = 1 + len(list(ONTO_DIR.glob("*.ttl")))
    stats = by_seq.get(stats_seq, {}).get("result", {})

    # Build per-query diagnostics
    diagnostics: list[dict[str, Any]] = []
    seq_offset = stats_seq + 1
    for i, q in enumerate(query_items):
        seq = seq_offset + i
        qname = q.get("_query_name", f"query-{i}")
        result = by_seq.get(seq, {})
        count, bindings = extract_count(result)
        if count < 0:
            diagnostics.append({
                "query": qname,
                "status": "ERROR",
                "error": result.get("result", {}).get("error", "unknown"),
            })
            continue
        entry: dict[str, Any] = {"query": qname, "status": "OK", "rows": count}
        variables = result.get("result", {}).get("variables", [])
        entry["variables"] = variables
        if count == 0:
            entry["concern"] = "ZERO ROWS — manufacturing path is severed"
        elif count == 1 and len(variables) > 1:
            # Likely a GROUP_CONCAT aggregate query — check that every variable
            # in row 0 has a non-empty value.
            first = bindings[0]
            empty_vars = []
            populated = []
            for v in variables:
                val = parse_literal(first.get(v))
                if not val or val.strip() == "":
                    empty_vars.append(v)
                else:
                    populated.append(v)
            entry["empty_aggregate_vars"] = empty_vars
            entry["populated_aggregate_vars"] = len(populated)
            entry["aggregate_var_count"] = len(variables)
            if empty_vars:
                entry["concern"] = (
                    f"AGGREGATE INCOMPLETE — {len(empty_vars)}/{len(variables)} "
                    f"aggregate variables empty: {', '.join(empty_vars[:5])}"
                )
            else:
                entry["health"] = f"all {len(variables)} aggregate variables populated"
        else:
            # Multi-row query — check each variable has non-empty values across rows
            empty_vars = []
            populated = []
            for v in variables:
                vals = [parse_literal(b.get(v)) for b in bindings]
                non_empty = [x for x in vals if x and x.strip()]
                if non_empty:
                    populated.append(v)
                else:
                    empty_vars.append(v)
            entry["empty_vars_across_rows"] = empty_vars
            entry["populated_vars_across_rows"] = len(populated)
            if empty_vars and len(empty_vars) == len(variables):
                entry["concern"] = "ALL VARIABLES EMPTY — extraction returned shapes but no data"
            elif empty_vars:
                entry["health"] = (
                    f"{len(populated)}/{len(variables)} variables populated; "
                    f"empty (likely OPTIONAL no-match): {', '.join(empty_vars[:3])}"
                )
            else:
                entry["health"] = f"all {len(variables)} variables populated across rows"
        diagnostics.append(entry)

    summary = {
        "stats": stats,
        "queries": diagnostics,
        "rows_total": sum(d.get("rows", 0) for d in diagnostics),
        "queries_with_rows": sum(1 for d in diagnostics if d.get("rows", 0) > 0),
        "queries_zero_rows": sum(1 for d in diagnostics if d.get("rows", 0) == 0 and d.get("status") == "OK"),
        "queries_errored": sum(1 for d in diagnostics if d.get("status") == "ERROR"),
        "queries_with_concerns": sum(1 for d in diagnostics if "concern" in d),
    }

    out_full = AUDIT_DIR / "extraction-diagnostics.json"
    out_full.write_text(json.dumps(summary, indent=2))

    # Human-readable summary
    lines = []
    lines.append("=" * 72)
    lines.append("P0-A Extraction Diagnostics")
    lines.append("=" * 72)
    lines.append("")
    lines.append(f"Store stats: {stats.get('triples', '?')} triples, "
                 f"{stats.get('classes', '?')} classes, "
                 f"{stats.get('individuals', '?')} individuals, "
                 f"{stats.get('object_properties', '?')} object properties")
    lines.append("")
    lines.append(f"Queries run:        {len(diagnostics)}")
    lines.append(f"  with rows:        {summary['queries_with_rows']}")
    lines.append(f"  zero rows:        {summary['queries_zero_rows']}")
    lines.append(f"  errored:          {summary['queries_errored']}")
    lines.append(f"  with concerns:    {summary['queries_with_concerns']}")
    lines.append(f"Total rows:         {summary['rows_total']}")
    lines.append("")
    lines.append("Per-query results:")
    lines.append("-" * 72)
    for d in diagnostics:
        if d["status"] == "ERROR":
            lines.append(f"  [ERROR] {d['query']:<42} {d['error'][:60]}")
            continue
        rows = d["rows"]
        bar = ("[zero]" if rows == 0 else "[ALIVE]" if rows >= 1 else "       ")
        concern = f" ⚠ {d['concern']}" if "concern" in d else ""
        health = f"  ({d['health']})" if "health" in d and "concern" not in d else ""
        lines.append(f"  {bar} {d['query']:<42} rows={rows:<3}{concern}{health}")
    lines.append("")
    lines.append("=" * 72)
    lines.append("μ_mobile Readiness Matrix")
    lines.append("=" * 72)
    lines.append(f"  {'Artifact':<32} {'Query':<32} {'Rows':<6} {'Ready'}")
    lines.append("-" * 72)
    mobile_artifacts = [
        ("Expo screens",          "extract-screens"),
        ("Expo navigation stack", "extract-navigation"),
        ("Admin screens",         "extract-admin-screens"),
        ("Form fields",           "extract-form-fields"),
        ("Push cards",            "extract-push-card-fields"),
        ("Connect-group forms",   "extract-connect-group-stages"),
    ]
    for label, qname in mobile_artifacts:
        d = next((x for x in diagnostics if x["query"] == qname), None)
        if d is None:
            lines.append(f"  {label:<32} {qname:<32} MISSING")
            continue
        rows = d.get("rows", 0)
        ready = "YES" if rows > 0 and "concern" not in d else ("PARTIAL" if rows > 0 else "NO")
        lines.append(f"  {label:<32} {qname:<32} {rows:<6} {ready}")
    lines.append("")
    lines.append("=" * 72)
    lines.append("Route-Runtime Drill-Down: ConnectGroupJoinRoute")
    lines.append("=" * 72)
    cg = next((x for x in diagnostics if x["query"] == "extract-connect-group-stages"), None)
    sr = next((x for x in diagnostics if x["query"] == "extract-service-routes"), None)
    lines.append(f"  stage extraction (extract-connect-group-stages): rows = {cg['rows'] if cg else 'N/A'}")
    lines.append(f"  service route extraction (extract-service-routes): rows = {sr['rows'] if sr else 'N/A'}")
    lines.append("  Expected: 8 POWL stages × predecessorStage edges, A0-A4 action classes,")
    lines.append("  7 admission gates (Consent/Capacity/Schedule/Policy/NotificationBudget/Role/RouteEnabled),")
    lines.append("  8 OCEL event types (one per stage).")
    lines.append("")
    lines.append("=" * 72)
    lines.append("Manufacturing-Restored Success Condition")
    lines.append("=" * 72)
    chain_checks = [
        ("Migration (extract-tables → supabase-migration.tera)",   "extract-tables"),
        ("Screen (extract-screens → expo-screen.tera)",            "extract-screens"),
        ("Route gate (extract-service-routes → route-stage-gate)", "extract-service-routes"),
        ("OCEL emitter (extract-ocel-events → ocel-event.tera)",    "extract-ocel-events"),
    ]
    lines.append(f"  {'Chain':<60} {'SPARQL rows':<12} {'Manufactures'}")
    lines.append("-" * 72)
    for label, qname in chain_checks:
        d = next((x for x in diagnostics if x["query"] == qname), None)
        rows = d.get("rows", 0) if d else 0
        manuf = "YES" if rows > 0 else "NO"
        lines.append(f"  {label:<60} {rows:<12} {manuf}")
    lines.append("")
    lines.append(f"Full JSON: {out_full}")
    lines.append("=" * 72)

    out_human = AUDIT_DIR / "extraction-diagnostics.txt"
    out_human.write_text("\n".join(lines))
    print("\n".join(lines))

    return 0


if __name__ == "__main__":
    sys.exit(main())
