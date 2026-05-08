---
name: Coding-Agent Mistakes — Mandatory Gate
description: 5 mistake classes, 6-question patch contract, invariants, sabotage tests
type: rules
---

# Coding-Agent Mistakes — Mandatory Gate

> **The strongest single rule:** Every coding-agent patch must either deepen authority or reduce drift.

This means: the patch must make the authoritative path harder to bypass, make bypasses fail loudly, or remove a bypass that already exists. A patch that adds a feature while leaving the old bypass intact does not satisfy this rule.

---

## 1. The Five Mistake Classes

### 1.1 Decorative Completion

**Definition:** A command exits 0 and prints success, but no durable state transition occurred. The world looks the same after the operation as before.

**open-ontologies examples:**
- `onto validate` prints "Validation passed" but `.onto/validation-cache/` is not written or is unchanged
- `ggen sync` emits "Sync complete" but `src/cmds/generated.rs` contains no new content and `.ggen/receipts/` is empty
- A receipt is created with an empty `signature` field (`""`).

**Detection:**
```bash
# Before/after diff on generated file
stat -f "%m" src/cmds/generated.rs   # macOS
stat -c "%Y" src/cmds/generated.rs   # Linux

# Receipt field check — signature must be non-empty
jq -e '.signature | length > 0' .ggen/receipts/latest.json

# Sabotage: if receipt is empty, validation report must be empty too
rm src/cmds/generated.rs && onto validate ontology/cli-open-ontologies.ttl
# Report must be empty or non-conforming (not "passed")
```

---

### 1.2 Epistemic Bypass

**Definition:** Logic that should be derived from the RDF ontology or SPARQL query is instead hardcoded inline. The codebase "knows" something it should only "ask."

**open-ontologies examples:**
- A tool handler matches command names with a `match` arm instead of querying the ontology
- A validation rule embeds SHACL shapes as string literals rather than loading them from `ontology/cell8-shapes.ttl`
- A Cell8 gate check hardcodes the list of 13 gates instead of deriving them from `ontology/cell8-manufacturing.ttl`

**Detection:**
```bash
# Find hardcoded command names outside ontology layer
grep -rn "\"validate\"\|\"query\"\|\"load\"" src/cmds/ --include='*.rs' | grep -v ontology | grep -v query

# Grep for inline TTL/shapes strings (should never appear outside test fixtures)
grep -rn 'sh:NodeShape\|sh:property' src/ --include='*.rs'

# Find hardcoded Cell8 gate list (should be derived from TTL)
grep -rn '"A1"\|"A2"\|"A3"' src/ --include='*.rs'
```

---

### 1.3 Fail-Open Behavior

**Definition:** The system continues executing when it should halt. A missing required resource produces a warning instead of an error; a violated constraint is logged but not enforced.

**open-ontologies examples:**
- A missing SHACL shapes file produces `warn!("shapes not found, skipping validation")` instead of returning `Err(...)`.
- A Cell8 gate violation appends a warning to the report but still marks the operation as passing
- `onto validate` returns exit 0 when the signature field is empty (should exit 1)

**Detection:**
```bash
# Verify that missing shapes triggers an error exit code, not a warning
ONTO_SHAPES_FILE=/tmp/nonexistent onto validate ontology/cli-open-ontologies.ttl
echo "exit: $?"  # must be non-zero

# Verify that signature validation fails hard when signature is empty
# (test in crates/ggen-receipt/tests/receipt_validation_test.rs)
cargo make test -- receipt_empty_signature
# Should exit non-zero if signature is empty
```

---

### 1.4 Legacy Path Contamination

**Definition:** A new authoritative path was built correctly, but the old bypass was not removed. Both paths coexist. The old path, being simpler, is hit in practice.

**open-ontologies examples:**
- `ggen sync` was refactored to read `.ggen/packs.lock`, but a fallback `load_packs_legacy()` function still executes when the lockfile is absent (fail-open + legacy contamination together)
- A new `ReceiptManager` was introduced but `write_raw_receipt()` still exists and is called from three non-test sites
- Two SHACL shape loading paths coexist: one from TTL, one hardcoded fallback

**Detection:**
```bash
# Find legacy function names still in non-test code
grep -rn 'load_packs_legacy\|write_raw_receipt' src/ --include='*.rs'

# Confirm new path is the only path (no dead fallback)
cargo make check 2>&1 | grep 'dead_code'

# Inspect call sites for dual-path routing
# Use LSP findReferences on the old function to enumerate callers
```

---

### 1.5 Contract Drift

**Definition:** The receipt, lockfile, or other proof object no longer accurately describes what actually ran. Fields are stale, absent, or populated with defaults that were never replaced.

**open-ontologies examples:**
- `.ggen/packs.lock` contains `"digest": ""` because the pack TOML was not hashed at install time
- A receipt records `input_hashes` from the previous sync run because the hash step was skipped on incremental re-run
- `operation_id` in a receipt is a hardcoded test UUID that was never replaced with a real `Uuid::new_v4()`

**Detection:**
```bash
# Check for empty or default-sentinel values in receipt
jq '.input_hashes[] | select(.digest == "" or .digest == null)' .ggen/receipts/latest.json

# Check for hardcoded UUIDs
grep -rn '00000000-0000-0000-0000-000000000000' .ggen/receipts/

# Verify receipt reflects current run's input hashes
# (run sync twice with different inputs; receipts must differ)
onto validate ontology/cli-open-ontologies.ttl && cp .ggen/receipts/latest.json /tmp/r1.json
touch ontology/cli-open-ontologies.ttl  # touch = update mtime
onto validate ontology/cli-open-ontologies.ttl && diff /tmp/r1.json .ggen/receipts/latest.json
# Must differ if file was modified
```

---

## 2. The Authoritative Path for open-ontologies

```
intent
  → ontology resolution      (ontology/*.ttl files, validation via onto validate)
  → SPARQL query execution   (.specify/queries/*.rq for ggen pipeline)
  → template rendering       (.specify/templates/*.tera)
  → code generation          (μ₁–μ₅ pipeline)
  → SHACL validation         (ontology/cell8-shapes.ttl gates)
  → artifact emission        (src/cmds/generated.rs, cell8-ggen/src/cell8/generated/)
  → receipt & cryptography   (.ggen/receipts/*.json, Ed25519 signature)
```

### What "touches" each stage

| Stage | Authoritative implementation | File/module |
|-------|------------------------------|-------------|
| Ontology resolution | Load from `ontology/` directory | `src/graph.rs` |
| SPARQL execution | Execute queries in `.specify/queries/` | `src/query.rs` |
| Template rendering | Tera templates in `.specify/templates/` | `src/codegen.rs` |
| Validation | SHACL shapes in `ontology/cell8-shapes.ttl` | `src/shacl.rs` |
| Generation | ggen μ₁–μ₅ pipeline | ggen crate |
| Receipt creation | `.ggen/receipts/` with signature | ggen crate |

### What "bypasses" each stage

| Bypass | Why it is forbidden |
|--------|---------------------|
| Hardcoding command/option names instead of querying ontology | Decorative completion — definitions are undefined |
| Embedding SHACL shapes as string literals | Epistemic bypass — ontology is not source of truth |
| Emitting artifacts before SHACL validation | Fail-open — violating constraints silently |
| Skipping receipt generation or signing | Contract drift — proof object is meaningless |
| Leaving `write_raw_receipt()` reachable after refactor | Legacy path contamination |

---

## 3. The 6-Question Patch Contract

Every agent patch must answer all six questions before the patch is accepted.

### Q1: What real state changed?

Not stdout. Name the file, database row, or in-memory structure that is different after this patch runs successfully.

> "`.ggen/receipts/latest.json` gains a non-empty `signature` field and `input_hashes` matching the ontology that was validated."

### Q2: What authoritative path did this patch touch?

Name the stage from Section 2.

> "Receipt creation stage — `ReceiptManager::emit()` now signs with Ed25519 before writing."

### Q3: What negative path now fails correctly?

Describe the sabotage condition and the expected error.

> "If the ontology file is deleted after validation, `ggen sync --locked` exits non-zero with `Error: ontology digest mismatch`."

### Q4: What invariant protects this patch from drift?

Reference an invariant from Section 4.

> "Receipt invariant: `signature` must be a non-empty base64 string. The serialization step enforces `!signature.is_empty()` or returns `Err`."

### Q5: What legacy path was removed or blocked?

If no legacy path was removed, explain why none exists. Silence on this question is a red flag.

> "`write_raw_receipt()` was deleted in this patch. Its three call sites were updated to use `ReceiptManager::emit()`."

### Q6: What proof object shows it worked?

Reference the receipt, test output, or OTEL span.

> "`.ggen/receipts/latest.json` has non-empty `signature` and `input_hashes` includes the validated ontology digest. `cargo make test -- receipt_signing` passes."

---

## 4. Invariant Definitions

### 4.1 Ontology Invariants

Every TTL file in `ontology/` must satisfy:

- All classes, properties, named individuals must have `rdfs:label` in English
- All `rdfs:subClassOf` and `rdfs:subPropertyOf` chains must be acyclic
- All `owl:TransitiveProperty` definitions must be explicitly declared (not inferred)
- No orphan definitions (property without domain and range)

### 4.2 Receipt Invariants

Every `.ggen/receipts/*.json` must satisfy:

```json
{
  "operation_id":   "<string: UUID v4, non-zero>",
  "timestamp":      "<string: RFC-3339>",
  "input_hashes":   { "<ontology-path>": "<sha256 hex>" },
  "output_hashes":  { "<output-file-path>": "<sha256 hex>" },
  "signature":      "<string: base64 Ed25519, non-empty>"
}
```

- `input_hashes` must include every ontology file consumed during the run
- `output_hashes` must include every artifact written
- `signature` must be produced by the private key in `.ggen/keys/signing.key`
- An empty `signature` field means the receipt is invalid — `ggen receipt verify` must return `is_valid: false`

### 4.3 Generation Invariants

1. `ggen sync` must read ontology files before executing any pipeline stage
2. `ggen sync` must emit a receipt after the artifact stage completes successfully
3. `ggen sync --locked` must fail hard (`exit 1`) if the ontology digest does not match
4. `ggen sync` must not write any output artifact if SHACL validation fails
5. The receipt emitted by sync must reflect the ontology set and input hashes of the current run, not a prior run

---

## 5. Sabotage Test Requirements

These tests must exist and pass. Each is a negative-path test proving the system fails loudly rather than silently.

| Sabotage | Command | Required outcome |
|----------|---------|-----------------|
| Remove ontology file after validation | `rm ontology/cli-open-ontologies.ttl && ggen sync --locked` | Exit non-zero; error message references digest mismatch or missing file |
| Corrupt receipt signature | `echo '{}' > .ggen/receipts/latest.json && ggen receipt verify` | `is_valid: false` in output |
| Delete verifying key | `rm .ggen/keys/verifying.key && ggen receipt verify` | `is_valid: false` or error; must not return `is_valid: true` |
| Missing SHACL shapes | `rm ontology/cell8-shapes.ttl && ggen sync` | Exit non-zero; error message references shapes not found |
| SHACL violation | `(modify TTL to violate shape) && ggen sync` | Exit non-zero; validation report shows violations |

---

## 6. The Strongest Single Rule

> **Every coding-agent patch must either deepen authority or reduce drift.**

### What this means for open-ontologies specifically

**Deepening authority** means the authoritative path (Section 2) becomes harder to bypass:
- A new `#[must_use]` return value forces callers to handle receipts
- A new typestate prevents calling `render()` before `validate()` compiles
- A new validation gate forces SPARQL query results through type checking
- A SPARQL query replaces an inline `Vec` of hardcoded values

**Reducing drift** means proof objects more accurately reflect what ran:
- `signature` is now computed after generation instead of defaulting to `""`
- `input_hashes` now include ontology versions instead of just filenames
- Receipt validation enforces non-empty signature before accepting

**A patch that does neither is noise at best, contamination at worst.** If you cannot answer "this deepens authority" or "this reduces drift" for your patch, stop and reconsider the approach before submitting.

### Quick self-check before submitting a patch

```
[ ] Q1 answered: real state change named (not just stdout)
[ ] Q2 answered: authoritative stage named (Section 2)
[ ] Q3 answered: sabotage condition described, exit code confirmed
[ ] Q4 answered: invariant referenced (Section 4)
[ ] Q5 answered: legacy path removed or confirmed absent
[ ] Q6 answered: receipt/test output cited as proof
[ ] Patch deepens authority OR reduces drift (Section 6)
```

If any box is unchecked, the patch is incomplete.

---

**See also:**
- [Andon Signals](../andon/signals.md) — stop-the-line protocol
- [OTEL Validation](otel-validation.md) — proof via spans for external services
- [ggen Pipeline](ggen-pipeline.md) — safe generation workflow
- `src/graph.rs` — ontology loading and validation
- `src/query.rs` — SPARQL execution
- `src/shacl.rs` — SHACL validation
