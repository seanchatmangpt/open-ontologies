# open-ontologies Gemini CLI Instructions

## Core Mandates for System Actuation and Proof

1. **Adversarial Truth**: Never write placeholders, mocks, or synthetic validation scripts (e.g., Python wrappers that just print "PASS"). Code must be physically implemented in the target architecture.
2. **The Ostar Generative Pipeline**: You MUST build the actual system capabilities using the `ostar-*` skills (`ostar-governor`, `ostar-architect`, `ostar-operator`, `ostar-doctor`, `ostar-auditor`).
   - DO NOT hand-write state machines or routing logic.
   - **Step 1 (Governor)**: Define the semantic law for the capability in the ontology (e.g., `.ttl` files) using the IES 4D pattern (State, Event, Consequence).
   - **Step 2 (Operator)**: Run `ggen sync` to scaffold the capability.
   - **Step 3 (Architect)**: Enforce the Chatman Equation (A = μ(O)) in the generated Rust/TypeScript code using zero-cost typestates.
   - **Step 4 (Auditor)**: Bind execution to OTel spans and BLAKE3 receipts.
3. **No Mocks, No Fakes, No Stubs**: Every validation, alignment, and receipt generation MUST invoke the actual underlying system logic.
4. **Use the Gemini CLI Skills**: The Gemini CLI must act as the actuation membrane, executing only what is defined in the ontology and synced by `ggen`. Do not bypass the pipeline.

# AutoReceipt Closure Law

AutoReceipt is not complete when expected OCEL exists.
AutoReceipt is not complete when a test clones expected OCEL into observed OCEL.
AutoReceipt is not complete when a receipt binds to "simulated execution."
AutoReceipt is not complete when a markdown architecture document is parsed and treated as execution.
AutoReceipt is not complete when only one integration test passes while any chained verification command fails.
AutoReceipt is not complete when the working tree is dirty and unclassified.
AutoReceipt is not complete when Gemini CLI executes in YOLO mode without an admitted ActuationPlan.

The only valid AutoReceipt closure form is:

R ⊢ A = μ(O*)

Where:
- O* is the lawful closure / expected route law
- μ is the real admitted execution operator
- A is the observed artifact/action/consequence
- R is a receipt derived from real boundary evidence
- ⊢ means the receipt admits/proves the relation

A receipt over synthetic execution is not an AutoReceipt.

## Required States

Use only these states for AutoReceipt:

- ExpectedOcelManufactured
- ExecutionBindingReady
- RealBoundaryExecuted
- ObservedOcelCaptured
- OcelAlignmentPassed
- OcelAlignmentFailed
- ReceiptEmitted
- ReceiptVerified
- AutoReceiptClosed
- AutoReceiptBlocked
- EvidenceIncomplete
- SyntheticObservedOcelRejected
- SyntheticClosureLie
- DirtyTreeUnclassified
- VersionMismatch
- CommandFailureUnresolved

## Hard Refusal Rules

If observed OCEL is cloned from expected OCEL, state must be SyntheticObservedOcelRejected.

If observed OCEL is generated from templates, state must be SyntheticObservedOcelRejected.

If execution_hash is derived from a literal such as "simulated execution", state must be SyntheticClosureLie.

If a command sequence includes any failing command, final state cannot be complete.

If git status is dirty and not fully printed, final state cannot be complete.

If Gemini CLI used YOLO/no-sandbox execution, the final proof must explicitly classify this as UnboundedActuation unless an OO ActuationPlan admitted the action first.

If any artifact contains hardcoded timestamps, hardcoded version strings, or placeholder execution boundaries, final state cannot be AutoReceiptClosed.

## Observed OCEL Requirements

Observed OCEL must be captured from real boundary execution.

Each observed OCEL file must include:
- command or harness id
- working directory
- exact command
- stdout hash
- stderr hash
- exit code
- started_at
- finished_at
- git_before
- git_after
- files_changed
- execution_receipt_hash
- boundary type
- actor basis, if applicable
- policy epoch
- proof hash

Observed OCEL must not be derived from expected OCEL.

## Alignment Requirements

Alignment may run only after observed OCEL is real.

Alignment must verify:
- required events exist
- required object references exist
- event order is lawful
- required boundary was crossed
- receipt exists
- receipt hash recomputes
- no synthetic evidence is used
- no false completion is projected
- closure state matches evidence

A structural string match of ocel:activity is not sufficient alignment.

## Receipt Requirements

A receipt must bind:
- O* hash
- μ / execution operator hash
- A / observed artifact hash
- expected OCEL hash
- observed OCEL hash
- alignment result hash
- prior receipt hash, if chained
- policy epoch
- projection basis, if role/purpose/scope/disclosure scoped

If the receipt does not bind real observed evidence, it is a smoke receipt, not AutoReceipt.

## Required Final Proof Block

Every final response must be emitted from disk artifacts, not written by narrative.

State:
<AutoReceiptClosed | AutoReceiptBlocked | EvidenceIncomplete | SyntheticClosureLie | CommandFailureUnresolved | DirtyTreeUnclassified | VersionMismatch>

Commit:
<git rev-parse HEAD>

Tree:
<full git status --short output>

Commands:
- <command>: pass/fail
- <command>: pass/fail

Counts:
- expected OCEL files:
- real observed OCEL files:
- synthetic observed OCEL files:
- alignment receipts:
- alignment passed:
- alignment failed:
- receipts emitted:
- receipts verified:
- synthetic closure rejected:
- unresolved command failures:

Artifacts:
- expected OCEL manifest: <hash>
- observed OCEL manifest: <hash>
- alignment manifest: <hash>
- receipt bundle: <hash>
- final proof report: <hash>

Verifier Output:
- real boundary execution: pass/fail
- synthetic observed OCEL rejection: pass/fail
- alignment verification: pass/fail
- receipt hash verification: pass/fail
- dirty tree classification: pass/fail
- version consistency: pass/fail
- command chain integrity: pass/fail

Remaining Blockers:
<exact blockers>

Next Command:
<single exact command>
