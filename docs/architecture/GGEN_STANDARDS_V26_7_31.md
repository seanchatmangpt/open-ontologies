# ggen v26.7.31 Standards Retrofit

## Preserve

Open Ontologies remains the Rust authority for OntoStar admission, typed defects, POWL/OCEL conformance, CellReady certification, provenance, tenant isolation, and receipt chains. Process-mining mathematics remains owned by wasm4pm. Models and cognition breeds remain candidate-producing capabilities without execution authority.

## Fence

This retrofit governs repository manufacture and verification. It does not claim that external systems were actuated, that production consequences were observed, or that a third party verified a release. BRCE remains the only lawful external actuation boundary.

The repository-wide WIP branch from pull request #4 is a separate object graph. This branch reused only its independently regenerated `Cargo.lock` blob after verifying the lock's package identities and exact wasm4pm source commit. No generated WIP corpus, local algorithm stub, database, build output, or archive was imported.

## Calculus

The retrofit installs four layers:

1. `AGENTS.md` defines constitutional authority and standing law.
2. `standards/ggen-v26.7.31.toml` makes the law machine-readable.
3. `tools/verify_ggen_standards.py` deterministically admits or refuses the repository contract.
4. GitHub workflows and Make targets execute the same locked verifier ladder on an exact revision.

Dependency portability is repaired by replacing absolute workstation paths with public or immutable locked sources:

- format/import types are supplied by `wasm4pm-compat` under the historical crate alias `wasm4pm-types`;
- `wasm4pm` and `wasm4pm-cognition` resolve to commit `f1d4d7ac8b2f9a0265be82991487766eb35b4675` through committed `Cargo.lock` and mandatory `--locked` verification;
- the unpublished `mcpp-core` adapter is explicitly `UNSUPPORTED` in clean public checkouts rather than resolved through `/Users/sac`.

## Exclusions

The following are not promoted by this change:

- external BRCE execution;
- production consequence observation;
- external attestation;
- production load or SLO claims;
- the `mcpp` feature;
- all-features standing while an unsupported private feature remains reserved;
- repository-wide `ALIVE` from one bounded workflow.

These surfaces remain `UNKNOWN` or `UNSUPPORTED` as declared in `RELEASE_STANDING.json` and the standards profile.

## Falsifier

The standards contract is refuted when any of the following is observed:

- `required_broker` is not `BRCE`;
- an absolute dependency path appears;
- the exact standing vocabulary changes;
- a generated or metadata-only surface promotes release standing;
- `Cargo.lock` contains a duplicate `(name, version, source)` identity;
- wasm4pm does not resolve to the admitted precise commit;
- CI invokes the deleted shell dead-parameter gate;
- a critical workflow has no named semantic and output owner;
- release standing is not `UNKNOWN` without admitted external evidence.

The Rust contract tests construct mutated temporary repositories to prove these refusals execute rather than merely exist in prose.

## Extension

A later feature PR may restore `mcpp` only after `mcpp-core` is published or vendored, its source is immutable, and real proof receipts cross the adapter. External release standing may advance only after BRCE intent, grant, actuation, consequence observation, receipt verification, and deterministic replay are bound to one exact revision.

## Operationalization

The bounded verifier ladder is:

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

A successful source and repository ladder proves only the bounded repository contract. External release standing remains `UNKNOWN` until the excluded consequence surfaces are independently observed.
