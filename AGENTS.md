# Open Ontologies Agent Constitution

This file is the sole normative agent contract for this repository. `CLAUDE.md`, `GEMINI.md`, editor rules, workflow prose, and generated documentation are subordinate projections. When they disagree with this file, this file governs.

## 1. Product boundary

Open Ontologies is the Rust ontology, admission, process-evidence, and receipt authority known as **OntoStar**. It is not a generic agent shell and it does not grant language models execution authority.

The manufacturing law is:

```text
A = μ(O*)
```

- `O` is an observation.
- `O*` is an admitted, bounded observation.
- `μ` is deterministic lawful manufacture.
- `A` is an artifact with bounded standing.

No artifact receives standing from prose, generation, model confidence, a zero exit code, or repository location alone.

## 2. Authority separation

The following distinctions are constitutional:

```text
candidate != verified != authorized != actuated
planning != actuation
model output != execution authority
intent hook != broker
command success != consequence success
```

External filesystem, process, network, cloud, deployment, credential, registry, and production mutation belongs only to **BRCE**, the admitted broker boundary. Ontology kernels, planners, cognition breeds, MCP handlers, POWL models, generated code, and autonomic controllers may manufacture typed intents. They MUST NOT directly perform external actuation unless operating inside an explicitly admitted BRCE adapter that emits an intent receipt before execution and a consequence receipt afterward.

There is no emergency, legacy, debug, test, administrator, or model-authority bypass around this rule.

## 3. Standing vocabulary

Repository status uses only these exact classifications:

- `ALIVE` — every required conjunctive evidence surface passed on one exact revision.
- `PARTIAL_ALIVE` — a bounded subset passed, while named obligations remain open.
- `BLOCKED` — an admitted external dependency or authority prevents execution.
- `BUILD_BROKEN` — the admitted source tree cannot parse, resolve, compile, or link at the claimed boundary.
- `UNKNOWN` — required observation has not occurred or has not been admitted.
- `UNSUPPORTED` — the requested capability is outside the implemented or authorized boundary.

Lifecycle and standing are orthogonal. `ACTIVE`, `DEPRECATED`, `RETIRED`, and `ARCHIVED` are lifecycle states, not evidence standings.

Generated source, synthetic fixtures, model output, self-assessment, and local configuration MUST NOT promote external or production standing. A generated release declaration remains `UNKNOWN` until live execution, receipt verification, consequence observation, and replay are admitted.

## 4. ALIVE evidence conjunction

A bounded capability may be classified `ALIVE` only when all five surfaces exist for the same exact revision and policy:

1. positive witness;
2. negative falsifier;
3. independent verifier;
4. receipt verification;
5. deterministic replay.

The crown is conjunctive. Weighted averages, percentages, aggregate scores, and broad workflow success MUST NOT conceal a missing surface or failed capability.

Receipts MUST bind the exact commit, Git tree, policy/configuration digest, input observation digest, output artifact digest, and predecessor receipt when a chain exists.

## 5. Operating process

Automatic and autonomic operation follows this process:

```text
O → O* → I → G → A → R → O'
```

- `O`: raw observation;
- `O*`: admitted bounded observation;
- `I`: deterministic intent;
- `G`: exact execution grant;
- `A`: atomic artifact-plus-receipt actuation through BRCE;
- `R`: causal operation receipt;
- `O'`: independently re-observed consequence.

Required controls:

- idempotency is consumed before actuation;
- retry is finite and deterministic;
- repeated failure opens a fail-closed circuit;
- Andon severity is monotonic and RED stops new execution;
- risky operations consume explicit error-budget units;
- rollback is a separately admitted inverse with its own receipt;
- MAPE-K repair has a hard cycle bound;
- failed, timed-out, unsupported, or unobserved operations never collapse into truth.

## 6. Ontology and generation authority

Authored semantic law belongs in public RDF vocabularies and bounded local profiles. Prefer RDF/RDFS, OWL, SHACL, PROV-O, DCAT, DCTERMS, SKOS, ODRL, FOAF, Schema.org, OCEL, QUDT, SOSA/SSN, and other established vocabularies before inventing local terms.

The lawful generation path is:

```text
admitted RDF
→ fail-closed SPARQL/SHACL gates
→ deterministic projections
→ generated consequences
→ independent byte verification
→ receipt
→ second manufacture
→ byte-identical replay
```

Generated files are not hand-authored. Every live generated output MUST have exactly one semantic owner and one load path. Generated outputs, receipts, caches, archives, fixtures, and authored constitutional files MUST remain visibly distinct.

Changes to generated files MUST be made at their ontology, query, template, or generator authority. Direct edits to protected generated files are refused.

Protected generated surfaces include at least:

- `src/cmds/generated.rs`;
- `cell8-ggen/src/cell8/generated/`;
- `docs/AUTOGEN/`;
- any path declared as generated by an admitted ggen manifest.

## 7. POWL, OCEL, MuStar, and cognition

POWL is process language, not a generic precedence list. Canonical POWL changes MUST preserve operator semantics, deterministic normalization, cycle refusal, executable projection, and conformance/replay compatibility.

OCEL evidence is derived from real execution traces. Unknown or dropped OntoStar spans MUST produce observable dropped-evidence signals. Release admission fails when required trace evidence is absent or a dropped-evidence budget is exceeded.

MuStar is the semantic planner/executor/refiner. GPT model tiers and cognition breeds are bounded capabilities used by MuStar; they do not define MuStar and they never own execution authority.

Agent and model outputs are candidate-only. Any proposed external operation MUST be compiled into a typed BRCE intent carrying authority, resource, tenant, trace, idempotency, and required-consequence evidence.

## 8. Dependency and build portability

A clean checkout is the build unit.

- Absolute workstation paths in manifests, scripts, workflows, tests, or generated configuration are prohibited.
- Git dependencies MUST resolve to immutable commit identities. This may be expressed by a manifest `rev`, or by the exact `precise` source recorded in committed `Cargo.lock` when every admitted command uses `--locked`.
- Registry dependencies MUST resolve through the committed lockfile.
- Private or unpublished dependencies MUST be vendored, published, or classified `UNSUPPORTED`; they MUST NOT be silently reached through a developer filesystem.
- `Cargo.lock` MUST parse and MUST NOT contain duplicate `(name, version, source)` package identities.
- Toolchain versions used for release evidence MUST be pinned.

A dependency graph that resolves only on one workstation is `BUILD_BROKEN`, not `ALIVE`.

## 9. Required verification commands

Run the smallest relevant gate while editing, then the complete bounded ladder before claiming completion.

```bash
python3 tools/verify_ggen_standards.py
cargo fmt --all -- --check
cargo metadata --locked --format-version 1 --no-deps
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --tests --no-fail-fast
make check
make adversarial
make cell8-certify
```

Feature-specific work MUST add its explicit feature set. `--all-features` is valid only when every feature dependency is portable and admitted.

Tests MUST exercise real code paths. Mocks, stubs, placeholders, synthetic telemetry, and fixture-only receipts may support unit isolation but MUST NOT serve as production standing evidence.

## 10. Git and change safety

- Work on a dedicated branch and draft pull request.
- Bind observations and receipts to the exact head under review.
- Do not rewrite shared history or force-push unless the user explicitly authorizes it.
- Do not mix unrelated WIP, generated archives, databases, build outputs, credentials, or worktree state into a bounded change.
- Preserve Chesterton fences: understand why a surface exists before deleting or replacing it.
- Separate inherited failures from regressions by reproducing the failure on the exact base revision.
- Never declare repository-wide success from a bounded component workflow.

## 11. Review structure

Substantial changes are documented in this order:

1. Preserve — existing law and behavior retained.
2. Fence — authority and object boundary.
3. Calculus — deterministic transformation performed.
4. Exclusions — capabilities intentionally not claimed.
5. Falsifier — negative evidence that would refute the claim.
6. Extension — lawful next checkpoints.
7. Operationalization — commands, workflows, receipts, replay, and standing.

## 12. Refusal rule

When required evidence is missing, contradictory, stale, untrusted, nonportable, or outside authority, refuse the promotion and emit the strongest accurate classification. No plausible narrative may substitute for an executable receipt.
