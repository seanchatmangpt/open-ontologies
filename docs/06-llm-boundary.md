# 06 — LLM Boundary

## The claim, made concrete

The pm4py paper *"From Natural Language to POWL via DSPy"* argues that an LLM, when *molded* by a typed signature and *gauged* by a structural validator, behaves as a **transducer** — a deterministic function from natural language into a constrained output space — not as an authority. OntoStar implements this claim verbatim across five LLM boundaries.

**Doctrine:** LLMs translate. Gates admit. Receipts prove.

The translator never writes to the triple store. It produces `CandidateCtq` / `CandidatePowl` / `CandidateProjection`. The deterministic admission gate is what *admits*, returning a typed `DefectClass` if the candidate fails any conjunct of `cell_ready`.

## SHACL-derived DSPy signatures

A `SignatureShape` (`src/signature_shape.rs`) is the **mold** the LLM fills. It carries:

1. **Field semantics.** Each input/output field has a `description` that the prompt builder embeds verbatim. The LLM sees the contract textually.
2. **Demos.** Few-shot input/output pairs constrain the shape *before* generation. This is the molding step.
3. **Validation.** After the LLM responds, every output field is checked against `required` / `min_len` / `allowed_values`. Failures surface a typed `ValidationFailure` which the refine loop uses to retry.

Shapes are derived from SHACL where possible (the SHACL property's `sh:datatype`, `sh:in`, `sh:minLength` map directly to `FieldSpec`). Where SHACL would be overkill (DSPy demos), the shape carries the constraint directly. The principle is symmetric: the LLM is shaped before it speaks and gauged after.

## The molded-LLM-as-transducer pattern

```text
natural language
     │
     ▼
 SignatureShape  ──────────►  Groq prompt (descriptions + demos embedded)
     │                              │
     │                              ▼
     │                         LLM response (provisional)
     │                              │
     ▼                              ▼
  validation gauges  ◄────────  parsed candidate
     │
     ├─ validation passes ──►  CandidateCtq / CandidatePowl emitted
     │
     └─ validation fails  ──►  refine loop retries with typed failure
                                (budget bounded; surfaces as audit-only deny)
```

The output of this pipeline is a candidate, not a fact. To become a fact it must pass `OntoStarAdmissionGate::evaluate(CtqAdmitted, ...)` or the equivalent gate for its operation type — which runs the full thirteen `cell_ready` conjuncts including POWL replay through wasm4pm.

## The pm4py POWL example

`tests/real_groq_powl.rs` replicates the pm4py paper's example end-to-end: a natural-language process description, a `SignatureShape` defining the POWL output field, a real Groq call (no mocks, no replay), then `wasm4pm::parse` as the structural gauge. Invalid POWL → typed `ValidationFailure` → refine loop. Five LLM boundaries got the same treatment in commits `1b7d6cc` and `619c3b1`:

| Boundary | Signature | Test |
|---|---|---|
| CTQ translation | candidate CTQ from voice | `tests/real_groq_ctq.rs` |
| Executive projection | ledger summary from receipts | `tests/real_groq_executive_projection.rs` |
| Plan workflow | candidate POWL from description | `tests/real_groq_plan_workflow.rs` |
| POWL refine | repair invalid POWL | `tests/real_groq_powl_refine.rs` |
| Solution spec | architecture skeleton | `tests/real_groq_solution_spec.rs` |

## Real-Groq integration

`src/llm_translator.rs` wraps Groq via `reqwest` with a strict secret-hygiene invariant (Invariant 7): the resolved API key lives only on the `GroqTranslator` struct and is bound to outbound requests via `bearer_auth`. It must never appear in OCEL events, receipts, requirements, work orders, counterfactual reports, or persisted prompts. `tests/secret_grep_ratchet.rs` enforces this via per-file alias tracking, tracing structured-field detection, and format-string identifier interpolation (Phase 6 Task E hardened the ratchet against three known bypass patterns).

`onto_translate_candidate` and `onto_executive_projection` (commit `c8d5588`) invoke the live Groq subprocess as MCP tools at the audit-only tier — they emit `admission_granted` for `LlmTranslate` op but the receipt records "candidate produced, no fact admitted."

## Chicago-TDD test approach

Commit `b5cdca7` ("real Groq at every human interaction point") encodes the rule: every place a human supplies natural language gets a real-Groq test that **does not mock the LLM**. Mocked LLM tests prove the test harness; real-Groq tests prove the boundary. Run with `--test-threads=1` to respect Groq's rate limit.

## Production engine selection

The three Groq-facing handlers (`onto_translate_candidate`,
`onto_executive_projection`, `onto_groq_status`) and the new
`GET /api/health/llm` HTTP route all dispatch through one engine
resolver — `config::resolve_llm_engine`. Two engines are wired:

- `inproc` — in-process `GroqTranslator` over `reqwest`. No python venv
  required. Suitable for environments that cannot ship the dspy / pm4py
  toolchain (small containers, FaaS).
- `groq_pm4py` — shells out to `scripts/ctq_from_voice.py`,
  `scripts/executive_projection.py`, `scripts/groq_status.py`. Uses dspy
  inside the chatmangpt/pm4py fork. Identical path proven by every
  `tests/real_groq_*` integration test.

### Precedence

| Rank | Source | How to set |
|------|--------|------------|
| 1 (highest) | Per-call `engine` argument on the MCP tool input | `{"engine": "groq_pm4py"}` |
| 2 | HTTP request header | `X-Ontostar-LLM-Engine: inproc` |
| 3 | `OPEN_ONTOLOGIES_LLM_ENGINE` env var | `export OPEN_ONTOLOGIES_LLM_ENGINE=groq_pm4py` |
| 4 | `[llm] engine = "..."` in config.toml | `engine = "inproc"` |
| 5 (lowest) | Auto-detect | API key resolvable → `groq_pm4py`, else `inproc` |

Invalid header / env values are silently dropped (the next-lower source
takes over). Unknown values via `--llm-engine` CLI flag fail fast — the
process refuses to start.

### CLI overrides

```bash
open-ontologies server serve-http --llm-engine groq_pm4py
open-ontologies server serve --llm-engine inproc --llm-python /opt/venv/bin/python
```

The flags set `OPEN_ONTOLOGIES_LLM_ENGINE` / `OPEN_ONTOLOGIES_LLM_PYTHON`
in the process environment before `Config::load`, so resolution is
uniform across stdio and HTTP transports.

### Health route

`GET /api/health/llm` returns
`{ ok, engine, model_reachable, key_present, model, error? }`. When the
resolved engine is `inproc` the route short-circuits without spawning a
subprocess — `model_reachable` is `false` but `ok` stays `true` because
the inproc engine has no remote probe to perform. `key_present` always
reflects whether `resolve_llm_api_key` returned `Some(_)`.

### Failure typing

Subprocess-induced denials surface as `LlmAuthorityClaimed { reason,
remediation }`. Recognised reasons: `"subprocess_unavailable"`,
`"key_invalid"`, `"timeout"`. The tag string is unchanged
(`"llm_authority_claimed"`); auditors that match on tags keep working.

## Translate-vs-admit ratio audit

`onto_translate_candidate` is **projection-only**. Its response JSON
carries `_projection_only: true` and the embedded `candidate` object is
provisional. No field of that JSON may be lifted into a receipt,
production record, or trust structure without first passing through the
deterministic admission gate (`onto_admit_ctq` / `OntoStarAdmissionGate`).

### The §7 LLMAuthority signal

`signature_shape::parse_and_validate` returns a `ParsedFields` struct
that pairs the admitted output map with a `llm_claimed_authority` flag.
The flag is set when the LLM's reply contains either of:

- `"provisional": false` — the LLM denies its output is provisional.
- `"authoritative": true` — the LLM asserts authority over its output.

The validator NEVER trusts the claim. It records it.
`onto_translate_candidate` emits an `llm_authority_claimed` OCEL event
**before** lifting the fields into a `CandidateCtq`, with attributes
`engine`, `defect_class = "llm_authority_claimed"`, and
`provisional_forced_to = "true"`. This makes the claim observable
independently of any downstream admission outcome — auditors querying
the OCEL log see the adversarial reply directly, rather than inferring
it from a defect-class stack trace.

### The translate-vs-admit ratio

Healthy operation: every `llm_candidate_translated` OCEL event is
followed (within the same scope) by either a `ctq_admitted` event or a
typed admission denial. A high ratio of `llm_candidate_translated`
events to `ctq_admitted` events indicates either:

1. The LLM is producing CTQs that fail admission deterministically
   (legitimate gauge work — gate is doing its job).
2. Callers are reading the projection JSON and acting on it without
   passing it through `onto_admit_ctq` (projection-as-authority bug —
   the projection-only contract is being violated).

A ratio query against the OCEL store can be added to monitor:

```sparql
SELECT (COUNT(?translated) AS ?translated_count)
       (COUNT(?admitted) AS ?admitted_count)
WHERE {
  ?translated a ocel:Event ; ocel:type "llm_candidate_translated" .
  OPTIONAL {
    ?admitted a ocel:Event ; ocel:type "ctq_admitted" .
  }
}
```

When the count of `llm_authority_claimed` events is non-zero, the
deployment is seeing adversarial LLM responses — the next remediation is
to harden the system prompt (`SignatureShape::compile_prompt`) or
escalate to the model's safety endpoint.

### Saboteur ratchets

Three test-layer ratchets pin the closure:

- `tests/llm_provisional_override.rs` — pins the detection logic.
  Adversarial JSON containing `"provisional": false` or
  `"authoritative": true` MUST flip `ParsedFields::llm_claimed_authority`.
- `tests/llm_authority_zero.rs` — lexical scan ensures no production
  module assigns LLM-output identifiers (`fields[...]`,
  `parsed.fields`, `candidate.ctq_text`, etc.) into authority
  structures (`Receipt`, `ProductionRecord`, `TrustedKeys`). Self-reference
  safe (forbidden patterns stored as byte arrays).
- `tests/hearsay_returns_typed_consensus.rs` — compile-time pin: the
  swarm fusion function `fuse_via_hearsay` MUST return `SwarmConsensus`,
  not `serde_json::Value`. Fails to compile on a return-type drift.
