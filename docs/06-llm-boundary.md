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
