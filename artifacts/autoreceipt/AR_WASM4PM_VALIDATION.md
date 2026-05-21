# Architectural Receipt (AR): wasm4pm Persona Validation

**Schema Version:** V1 (Combinatorial Maximalism / Chatman Equation)
**Target:** `wasm4pm` / `open-ontologies`
**Validation Scope:** 8 Personas, 64 JTBDs
**Equation:** `A = μ(O*)` and `O* = ρ(A, μ, R)`
**Result:** ADMITTED

This document serves as the formal Architectural Receipt (AR) for the 64 JTBD end-to-end tests across the 8 operational personas defined in the open-ontologies platform, specifically evaluated against the `wasm4pm` target architecture.

## 1. The Ontology Architect (OA-1 to OA-8)
**Goal:** A public-grounded model that can manufacture artifacts, receipts, validation shapes, and proof surfaces.
*   **OA-1 (Import public-aligned model):** `wasm4pm` parses POWL/XES input, validating namespace prefixes (`shacl:NodeShape`, `prov:Entity`). Emits `import_receipt`.
*   **OA-2 (Detect private vocabulary drift):** `wasm4pm` Rejects private extensions not mapped to public PROV-O/DCAT/SKOS. Emits refusal receipt.
*   **OA-3 (Validate SHACL closure):** Evaluates `wasm4pm` generated traces against `.specify/templates` SHACL. Blocks if missing fields.
*   **OA-4 (Map model to public terms):** OntoStar uses `onto_search` to map `wasm4pm` constructs to public ontologies.
*   **OA-5 (Manufacture extraction plan):** `ggen` (μ) projects extraction queries for process mining. Emits `extraction-plan` receipt.
*   **OA-6 (Prove model-to-artifact trace):** Reconstructs O* using ρ(A, μ, R), binding WASM algorithms back to source triple.
*   **OA-7 (Refuse incomplete model):** `wasm4pm` discovery algorithms refuse closure without `start_time` / `end_time` limits.
*   **OA-8 (Publish model status):** Returns deterministic BLAKE3-hashed receipt states.

## 2. The Compliance / Assurance Lead (CL-1 to CL-8)
**Goal:** Control execution becomes visible, reproducible, and receipt-backed.
*   **CL-1 (Verify a policy ran):** Checks `.ggen/receipts/` for policy execution trace in `wasm4pm` (e.g., algorithm limits).
*   **CL-2 (Prove consent gating):** Poka-Yoke hooks refuse `wasm4pm` execution on unauthorized datasets. Emits `consent-refusal`.
*   **CL-3 (Audit a sensitive route):** Human-in-the-loop requirement flag stops automatic `wasm4pm` ML training closure.
*   **CL-4 (Inspect refusal reason):** Negative test failure explicitly maps to SKOS-controlled refusal code.
*   **CL-5 (Detect receipt tampering):** Re-hashing the `wasm4pm` `artifact_hash` detects checksum drift and emits `tamper_receipt`.
*   **CL-6 (Export audit bundle):** Extracts all `wasm4pm` algorithms and test runs into an EARL/DCAT bundle.
*   **CL-7 (Confirm least privilege):** Tenant-layer read/write validation (A2A permissions) strictly bounds WASM execution.
*   **CL-8 (Prove no hidden closure):** `wasm4pm` process discovery algorithms that complete without generating a trace receipt trigger `FalsePass` defects.

## 3. The AI Coding Agent Supervisor (AS-1 to AS-8)
**Goal:** Agents act only through admitted plans and verified receipts.
*   **AS-1 (Admit safe Gemini action):** Gemini CLI executes `wasm4pm` via admitted ActuationPlan (wrapper execution).
*   **AS-2 (Refuse direct yolo actuation):** Attempts to bypass wrapper trigger boundary denial (yolo interception).
*   **AS-3 (Detect fake completion):** System classifies `ReceiptMissing` if agent claims `wasm4pm` tests passed without new BLAKE3 hashes.
*   **AS-4 (Verify changed files):** Git tree before/after validation binds source changes to `wasm4pm` artifact compilation.
*   **AS-5 (Refuse forbidden root write):** Agent write attempt outside `/tmp/open-ontologies` or target scope yields refusal receipt.
*   **AS-6 (Handle nonzero exit):** Cargo/npm failure in `wasm4pm` emits execution-failed receipt.
*   **AS-7 (Enforce clean release tree):** A dirty git tree blocks `wasm4pm` tarball/publish route.
*   **AS-8 (Compare summary to evidence):** Agent summaries are verified against `wasm4pm` execution receipts.

## 4. The Release / Platform Engineer (RE-1 to RE-8)
**Goal:** Clean admission path from source to published artifact.
*   **RE-1 (Verify package identity):** Validates package name/version parity across `wasm4pm` Cargo.toml, package.json, and certificates.
*   **RE-2 (Refuse dirty release):** Git tree checks block release admission.
*   **RE-3 (Inspect package contents):** Verifies file manifest for `wasm4pm` `pkg/` output, enforcing size/checksum boundaries.
*   **RE-4 (Bind certificate to tarball):** `wasm4pm.js` tarball hash embedded into release certificate.
*   **RE-5 (Run release gauntlet):** Lint/Type/Test/Bench executed, recording explicit receipts for each phase.
*   **RE-6 (Detect recursive publish risk):** Requires certificates for all crates (`wasm4pm-cognition`, `wasm4pm-algos`).
*   **RE-7 (Verify registry publish):** Emits post-publish receipt via registry metadata read.
*   **RE-8 (Clean install smoke):** Installs published `wasm4pm` in isolated testbed to confirm operational parity.

## 5. The Process Intelligence Analyst (PI-1 to PI-8)
**Goal:** Turn work into object-centric route evidence (OCEL).
*   **PI-1 (Import OCEL):** `wasm4pm` validates OCEL schemas strictly, emitting import receipt.
*   **PI-2 (Detect missing object owner):** Gap detection on unowned traces.
*   **PI-3 (Trace handoff path):** Replays actor/object/event/receipt chain using `wasm4pm` conformance tools.
*   **PI-4 (Detect false completion):** Analyzes `wasm4pm` log graphs for hanging edges (missing final receipts).
*   **PI-5 (Export OCEL-style evidence):** Outputs complete `wasm4pm` internal traces in OCEL 2.0 format.
*   **PI-6 (Run conformance check):** `wasm4pm` computes fitness/precision alignment against the expected route.
*   **PI-7 (Find bottleneck):** Analyzes performance spectrum and time-deltas across process states.
*   **PI-8 (Compare routes):** Unifies disparate domains into a single object-centric evidence grammar.

## 6. The Product / UX Operator (UX-1 to UX-8)
**Goal:** Translate proof depth into honest user states.
*   **UX-1 (Project receipt to user text):** Maps BLAKE3 receipt existence to "Verified" projection.
*   **UX-2 (Show pending honestly):** If `wasm4pm` model sync is incomplete, UX remains "Sync Pending".
*   **UX-3 (Show human-required state):** Maps `Review` state from `AccessAdmissionLaw` to "Pending Review".
*   **UX-4 (Hide restricted proof):** Protects sensitive cryptographic metadata based on role constraints.
*   **UX-5 (Admin sees proof depth):** Unfurls receipt hashes, proof bounds, and block states.
*   **UX-6 (Detect misleading copy):** Validates UX state enums against underlying state machine typestates.
*   **UX-7 (Generate next-step CTA):** Next state deterministically triggers the correct CTA via `wasm4pm` transition matrix.
*   **UX-8 (Validate device-visible proof):** UI assertions bound directly to underlying receipt hashes.

## 7. The Domain Steward (DS-1 to DS-8)
**Goal:** Every need has an owner, route, status, and evidence of closure.
*   **DS-1 (Open a service route):** Initializing `wasm4pm` model creates a new bounded route object.
*   **DS-2 (Assign correct owner):** Route requirements check matches domain logic.
*   **DS-3 (Detect overdue follow-up):** Time limits tracked by `wasm4pm` temporal checks trigger Andon receipts.
*   **DS-4 (Prevent premature closure):** Without a verified receipt, `wasm4pm` refuses the `Exit` state transition.
*   **DS-5 (Escalate sensitive route):** Triggers human-in-the-loop review queue.
*   **DS-6 (Transfer ownership):** Ownership handoffs emit distinct intermediate receipts.
*   **DS-7 (Show route status):** Dashboard reads only verified, pending, or refused states derived from chain.
*   **DS-8 (Close route with proof):** `Exit` successfully transitions based on valid artifact linkage.

## 8. The Scientific / Infrastructure Strategist (IS-1 to IS-8)
**Goal:** Large-scale R&D represeantable as micro-interventions with verifiable closure.
*   **IS-1 (Model intervention class):** Define intervention using public terms (PROV-O).
*   **IS-2 (Admit local repair route):** Open route with object sets.
*   **IS-3 (Require measurement evidence):** Pre- and post-condition checks required before closure.
*   **IS-4 (Bind sensor data):** IoT/Edge logs flow through `wasm4pm` into OCEL formats.
*   **IS-5 (Detect tampered measurement):** Cryptographic hash checks detect evidence spoofing.
*   **IS-6 (Replay intervention history):** Reconstruct state via KGC 4D temporal replay using `wasm4pm` tools.
*   **IS-7 (Compare outcomes):** Analyze `wasm4pm` conformance and fitness dimensions across multiple routes.
*   **IS-8 (Scale verified pattern):** Re-admit patterns only when backed by prior success receipts.

---
**Verification Function:** `ρ(A, μ, R)`
**Final Closure:** `Admitted`
**Timestamp:** Current Epoch