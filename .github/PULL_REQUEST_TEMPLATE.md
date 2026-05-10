# Pull Request

## Summary

<!-- Brief description of what this PR does and why. -->

## Authoritative-path / drift impact

<!-- Per coding-agent-mistakes.md: every patch must deepen authority OR reduce drift. -->
<!-- Which authoritative stage does this touch? What legacy bypass (if any) was removed? -->

## Verification matrix checklist

- [ ] tests added or updated
- [ ] no new `#[ignore]` introduced
- [ ] no string-literal `evaluate_admission(` bypass
- [ ] DefectClass taxonomy unchanged or version bumped + hash repinned
- [ ] real toolchain regression (cargo + erlc + terraform) green
- [ ] receipt chain invariants (per-session sequence, atomic persist+emit) preserved

## CI status

- [ ] `lib-unit` matrix green (macOS + ubuntu, stable + nightly)
- [ ] all `integration-suites` green
- [ ] `real-toolchain-smoke` green
- [ ] `dead-param-gate` green
- [ ] `ratchet-sweep` green
- [ ] `real-groq-sweep` ran (or correctly skipped when secret absent)
- [ ] No LLM/AI action introduced in CI (cascade is V1 deterministic)

## Notes for reviewers

<!-- Anything reviewers should look at carefully: invariants changed, taxonomy bumps, etc. -->
