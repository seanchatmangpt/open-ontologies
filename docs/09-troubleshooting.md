# 09 — Troubleshooting

Known failure modes and their causes. Most are documented Andon signals or fix-forward decisions, not bugs.

## `cargo test --test cli_test` shows stale failures

**Symptom.** `cargo test --test cli_test` is red despite recent commits showing a fix.

**Cause.** The CLI tests shell out to a built `onto` binary in `target/`. The subprocess inherits stale state when:

- A previous build left a `target/debug/onto` binary that predates the source changes (cargo's incremental detection misses subprocess-launched binaries).
- An admission state DB from a prior run lingers in the test's `--data-dir`.

**Fix.** Force a clean test binary:

```bash
cargo clean -p open-ontologies
cargo build --release --bin onto
cargo test --test cli_test
```

The `oo_isolated()` helper in `tests/cli_test.rs` appends `--data-dir` *after* the verb (this was the Phase 6 Task B fix in commit `0527773`). If you see "unrecognized argument --data-dir", you are running pre-Task-B code.

## `cargo test --test adversarial_jtbd_test` red on a fresh checkout

**Symptom.** Five JTBD tests fail with "unknown subcommand."

**Cause.** Same root cause as above — the noun-verb refactor at commit `362fd6b` broke flat verbs. Use `ontology query` not `query`, `ontology version` not `governance version`, `ontology sparql` not `ontology query` for SPARQL specifically.

**Fix.** Phase 6 Task B (commit `0527773`) is the canonical mapping table. If the test source predates that commit, rebase first.

## A `#[ignore]` is present in admission tests

**Symptom.** `cargo test` reports "1 ignored" or similar.

**Cause.** Audit ignore IS the Andon protocol. The Phase 6 audit found four legitimately-ignored tests in `tests/admission.rs` because `POWLReplayPass` is conjunct #4 in `cell_ready` and fires *before* `RequiredStagesPresent` (#6). Tests that emit a partial trace specifically to trigger `CapabilityZero` would now hit `ReplayFailed` first under real replay.

**The correct response.** Read the `// INTENTIONAL: gate-semantics test, see plan §A` comment. Do not unignore. The four affected tests are `skipped_stage_denial`, `wrong_order_denial`, `bypass_revokes_subsequent_operations`, `replay_enforcement_after_corruption`. Phase 7 (commit `0ab7577`) closed all four happy-path `#[ignore]` tags; what remains is intentional.

## `data push` fails with a Tokio runtime error

**Symptom.** `onto data push --endpoint ...` panics with `there is no reactor running, must be called from the context of a Tokio 1.x runtime`.

**Cause.** `push` issues HTTP via `reqwest`. The CLI command harness must wrap the verb body in a `tokio::runtime::Runtime::new()?.block_on(...)`. If the command was wired without that wrapper, push will panic on first request.

**Fix.** The Phase 7 happy-path-push closure (commit `0ab7577`) added the wrapper. Ensure your `src/cmds/data.rs::push` uses `tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async { ... })`.

## `GROQ_API_KEY` is not picked up

**Symptom.** Real-Groq tests skip or report "no API key."

**Cause.** The translator resolves the key in this order:

1. `[llm] api_key = "..."` in `config.toml`
2. `GROQ_API_KEY` environment variable
3. `.env` file in the working directory

The `.env` loader only fires if the working directory contains it. CI runners and `cargo test` do not auto-load `.env` from a parent directory.

**Fix.** Either export `GROQ_API_KEY` explicitly (`export GROQ_API_KEY=...`) or set it in `config.toml` (which is git-ignored — see `.gitignore`).

## Secret-grep ratchet fails on a benign change

**Symptom.** `cargo test --test secret_grep_ratchet` rejects a commit that mentions `api_key` in a comment or test fixture.

**Cause.** Phase 6 Task E (commit `063d540`) hardened the ratchet against three known bypass patterns: per-file alias tracking (`let X = api_key`), tracing structured-field detection (`?api_key`, `%api_key`), format-string identifier interpolation (`{api_key}`, `{api_key:?}`). The hardened scanner is intentionally noisy.

**Fix.** Use a different identifier for fixtures (e.g. `placeholder_token`). Comments containing the literal string `api_key` are caught. This is by design — a commenter could leak a real key.

## `onto verify` reports `is_valid: false` on a known-good bundle

**Symptom.** Verifier rejects a bundle you just produced.

**Cause.** Most likely, the carrier format's strip-and-rehash failed because:

- The inline header contains a line that doesn't match `^<prefix> ostar-[a-z-]+: .+$` (e.g. blank line within the block).
- The Terraform bundle is missing `iac/.ontostar-receipt.json` (sidecar required for closed-schema formats; commit `c4e0035`).
- The `defects_taxonomy_version` in the receipt does not match the verifier binary's pinned constant — taxonomy bumps are breaking changes (commit `bea21b4` bumped to `3.0.0`).

**Fix.** Run with `RUST_LOG=trace cargo run --bin onto -- verify ./bundle/` and read the trace span attributes — the verifier emits `verify.strip_failed` or `verify.taxonomy_mismatch` with the offending field.

## `make adversarial` complains about `let _ = param`

**Symptom.** Build fails with "dead param" Andon signal.

**Cause.** `let _ = param;` is theatre code per `CLAUDE.md` absolute rule §4. The dead-param gate (`tools/dead-param-gate.sh`) refuses to allow it.

**Fix.** Remove the parameter from the function signature, update callers. Do not add `#[allow(unused_variables)]` — the gate detects that too.
