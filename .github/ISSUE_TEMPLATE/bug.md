---
name: Bug report
about: Report a defect with reproduction steps and receipt-chain evidence
title: "[bug] "
labels: ["bug"]
assignees: []
---

## Summary

<!-- One-sentence description of the defect. -->

## Environment

- OS / arch:
- Rust toolchain (`rustc --version`):
- Branch / commit:
- Feature flags / env vars set:

## Reproduction steps

1.
2.
3.

```bash
# Exact commands that reproduce the issue
```

## Expected behavior

<!-- What should have happened. Cite the invariant (Section 4 of coding-agent-mistakes.md) being violated. -->

## Actual behavior

<!-- What actually happened. Include error messages verbatim. -->

## Receipt chain dump

<!-- Required for any defect involving manufacturing pipeline, admission, or generation. -->
<!-- Paste the relevant `.ggen/receipts/*.json` (or session receipt chain) inside the fence. -->

```json
```

## Logs / OTEL spans

<!-- Run with RUST_LOG=trace,onto=trace,ggen=trace and paste relevant span output. -->

```
```

## Suspected root cause (optional)

<!-- Which mistake class (decorative completion, epistemic bypass, fail-open,
     legacy path contamination, contract drift)? -->
