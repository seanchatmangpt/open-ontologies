# open-ontologies Human Persona Suite

The open-ontologies platform operates as a consequence control system for 8 distinct operational personas. Each persona requires 8 dimensions of proof (Public alignment, Policy admission, Actuation, Positive evidence, Negative evidence, Receipt, Projection, Closure/refusal).

Below are the 8 personas and their 64 Jobs-to-be-Done (JTBD) end-to-end tests.

## Persona 1 — The Ontology Architect

### Profile
The Ontology Architect is responsible for turning messy domain knowledge into public-aligned structure that can drive code, receipts, policies, tests, routes, and UI. They are usually a senior architect, knowledge engineer, platform architect, or advanced systems designer.

### Core anxiety
“If the source model is weak, every downstream artifact will drift.”

### Desired outcome
A public-grounded model that can manufacture artifacts, receipts, validation shapes, and proof surfaces without private vocabulary drift.

### Success condition
The architect can submit a domain model and see whether it is public-aligned, complete, validated, and ready for downstream manufacture.

### 8 JTBD end-to-end tests

| ID   | Job                             | End-to-end proof                                                                                                       |
| ---- | ------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| OA-1 | Import a public-aligned model   | User uploads TTL → OO parses RDF → validates prefixes → rejects private predicates → emits model admission receipt     |
| OA-2 | Detect private vocabulary drift | User adds product-specific classes → OO flags violation → returns public replacement mapping → emits refusal receipt   |
| OA-3 | Validate SHACL closure          | User runs validation → OO applies shapes → reports missing required fields → blocks artifact manufacture               |
| OA-4 | Map model to public terms       | User asks “what public vocabularies represent this?” → OO maps to PROV-O/DCAT/SKOS/ODRL/etc. → emits alignment receipt |
| OA-5 | Manufacture extraction plan     | User requests downstream artifacts → OO identifies SPARQL queries/templates needed → emits extraction-plan receipt     |
| OA-6 | Prove model-to-artifact trace   | User inspects one emitted artifact → OO shows source triple/query/template/output binding                              |
| OA-7 | Refuse incomplete model         | User attempts sync with missing required object relations → OO refuses before actuation                                |
| OA-8 | Publish model status            | User asks “is this ready?” → OO returns admitted/blocked state with receipts, not prose                                |

---

## Persona 2 — The Compliance / Assurance Lead

### Profile
The Compliance Lead needs proof that policies, controls, and refusals actually executed. They care about auditability, evidence retention, data boundaries, consent, refusal states, and traceability.

### Core anxiety
“People will say the control worked, but I need evidence that it actually ran.”

### Desired outcome
Control execution becomes visible, reproducible, and receipt-backed.

### Success condition
The Compliance Lead can trace any outcome back to policy, actor, evidence, and receipt.

### 8 JTBD end-to-end tests

| ID   | Job                      | End-to-end proof                                                                               |
| ---- | ------------------------ | ---------------------------------------------------------------------------------------------- |
| CL-1 | Verify a policy ran      | User selects policy → OO shows execution activity, inputs, decision, receipt hash              |
| CL-2 | Prove consent gating     | Missing consent request enters system → OO refuses action → emits consent-refusal receipt      |
| CL-3 | Audit a sensitive route  | Sensitive care/action route enters → OO classifies human-required → prevents automatic closure |
| CL-4 | Inspect refusal reason   | User opens blocked action → OO shows typed refusal code, policy source, and evidence           |
| CL-5 | Detect receipt tampering | Receipt hash modified → OO recomputes hash → detects mismatch → emits tamper receipt           |
| CL-6 | Export audit bundle      | User requests audit package → OO emits evidence dataset, receipt chain, checksum manifest      |
| CL-7 | Confirm least privilege  | Tool requests write/network/shell access → OO checks permission matrix → allows/refuses        |
| CL-8 | Prove no hidden closure  | Agent says “done” without receipt → OO refuses closure and records false-completion attempt    |

---

## Persona 3 — The AI Coding Agent Supervisor

### Profile
The Agent Supervisor manages Claude Code, Gemini CLI, Cursor, Codex, or other coding agents. They are often a founder, tech lead, staff engineer, or platform owner.

### Core anxiety
“Agents produce plausible work and convincing summaries before the system is actually closed.”

### Desired outcome
Agents can act only through admitted plans, controlled actuation, and verified receipts.

### Success condition
The supervisor can delegate safely because OO forces every agent action through policy, evidence, and refusal gates.

### 8 JTBD end-to-end tests

| ID   | Job                          | End-to-end proof                                                                               |
| ---- | ---------------------------- | ---------------------------------------------------------------------------------------------- |
| AS-1 | Admit safe Gemini action     | OO emits ActuationPlan → Gemini CLI executes through wrapper → receipt emitted → OO verifies   |
| AS-2 | Refuse direct yolo actuation | Gemini tries direct `--approval-mode yolo` without plan → OO refuses before execution          |
| AS-3 | Detect fake completion       | Agent claims work complete but no files/receipts changed → OO classifies ReceiptMissing        |
| AS-4 | Verify changed files         | Agent edits files → OO captures git_before/git_after → verifies declared file set              |
| AS-5 | Refuse forbidden root write  | Agent attempts write outside allowed path → OO blocks execution → refusal receipt emitted      |
| AS-6 | Handle nonzero exit          | Admitted command exits nonzero → OO emits execution-failed receipt → closure refused           |
| AS-7 | Enforce clean release tree   | Agent attempts publish with dirty tree → OO blocks publish route                               |
| AS-8 | Compare summary to evidence  | Agent summary says “all tests passed” → OO checks command receipts → accepts or flags mismatch |

---

## Persona 4 — The Release / Platform Engineer

### Profile
The Release Engineer owns package identity, versions, CI/CD, npm/cargo publication, artifact contents, release certificates, and post-publish proof.

### Core anxiety
“The package may publish with the wrong name, wrong files, stale version, missing proof, or no install verification.”

### Desired outcome
Every release has a clean admission path from source to published artifact.

### Success condition
A release cannot be called closed until source, tests, package artifact, registry result, and post-publish install all verify.

### 8 JTBD end-to-end tests

| ID   | Job                           | End-to-end proof                                                                          |
| ---- | ----------------------------- | ----------------------------------------------------------------------------------------- |
| RE-1 | Verify package identity       | OO compares package name/version across source, lockfile, certificate, tarball            |
| RE-2 | Refuse dirty release          | Git tree dirty before release → OO blocks release admission                               |
| RE-3 | Inspect package contents      | Pack artifact created → OO checks file list, size, forbidden files, checksums             |
| RE-4 | Bind certificate to tarball   | OO hashes tarball → embeds hash in release certificate → verifier recomputes              |
| RE-5 | Run release gauntlet          | OO runs lint/type/test/examples/behavior gates → records pass/fail receipts               |
| RE-6 | Detect recursive publish risk | Agent attempts workspace-wide publish → OO refuses unless every package has certificate   |
| RE-7 | Verify registry publish       | Package published → OO reads registry metadata/integrity → emits post-publish receipt     |
| RE-8 | Clean install smoke           | OO installs package in temp project → runs import/CLI smoke → closes release only if pass |

---

## Persona 5 — The Process Intelligence Analyst

### Profile
The Process Analyst studies workflows, event logs, route movement, bottlenecks, handoffs, false completions, and object-centric evidence.

### Core anxiety
“Traditional logs hide the real process because work involves many objects, people, systems, and receipts.”

### Desired outcome
OO turns work into object-centric route evidence that can be mined, replayed, and audited.

### Success condition
The analyst can see what moved, what stalled, what failed, and what only appeared complete.

### 8 JTBD end-to-end tests

| ID   | Job                           | End-to-end proof                                                                         |
| ---- | ----------------------------- | ---------------------------------------------------------------------------------------- |
| PI-1 | Import object-centric log     | User imports event data → OO validates object/event relationships → emits import receipt |
| PI-2 | Detect missing object owner   | Route/event lacks owner → OO flags unowned gap → blocks closure                          |
| PI-3 | Trace handoff path            | User selects route → OO shows actor/object/event/receipt chain                           |
| PI-4 | Detect false completion       | Route marked complete but missing final receipt → OO classifies false completion         |
| PI-5 | Export OCEL-style evidence    | User exports process evidence → OO emits event/object dataset + checksum                 |
| PI-6 | Run conformance check         | Process expected route vs actual route → OO identifies skipped/misordered steps          |
| PI-7 | Find bottleneck               | OO computes wait time between route states → highlights stuck stage                      |
| PI-8 | Compare routes across domains | Prayer route, release route, and repair route are shown through same evidence grammar    |

---

## Persona 6 — The Product / UX Operator

### Profile
The Product/UX Operator turns complex routing, proof, and policy states into clear user-facing experiences. They may be a product manager, UX designer, ministry admin, service designer, or ops coordinator.

### Core anxiety
“The proof system will become too technical for users, and users will not know what to do next.”

### Desired outcome
Users see simple, accurate next-step statuses while operators retain proof depth.

### Success condition
The UX surface never lies, never overexposes technical internals, and always gives the user the next right action.

### 8 JTBD end-to-end tests

| ID   | Job                           | End-to-end proof                                                                       |
| ---- | ----------------------------- | -------------------------------------------------------------------------------------- |
| UX-1 | Project receipt to user text  | Verified receipt → UI shows “Received” or “Connected,” not internal hash jargon        |
| UX-2 | Show pending honestly         | Local-only route → UI shows “Sync pending,” not “complete”                             |
| UX-3 | Show human-required state     | Sensitive route → UI shows “Pending review” with clear expectation                     |
| UX-4 | Hide restricted proof         | Unauthorized user opens route → OO projects safe summary or hidden-by-policy           |
| UX-5 | Admin sees proof depth        | Admin opens same route → sees receipt type, boundary, blocker, hash preview            |
| UX-6 | Detect misleading copy        | UI copy says “complete” while receipt pending → OO flags copy/proof mismatch           |
| UX-7 | Generate next-step CTA        | Route state changes → OO projects appropriate CTA: follow up, review, message, close   |
| UX-8 | Validate device-visible proof | Maestro/Detox flow asserts visible state → verifier binds UI assertion to receipt hash |

---

## Persona 7 — The Domain Steward

### Profile
The Domain Steward owns real-world consequence: people, care, resources, environmental interventions, school support, healthcare routes, church follow-up, civic service, or nonprofit operations.

### Core anxiety
“People and needs disappear into gaps while reports say everything is handled.”

### Desired outcome
Every need has an owner, route, status, follow-up, and evidence of closure or refusal.

### Success condition
No person, request, or intervention can vanish without a visible state and accountable next step.

### 8 JTBD end-to-end tests

| ID   | Job                          | End-to-end proof                                                              |
| ---- | ---------------------------- | ----------------------------------------------------------------------------- |
| DS-1 | Open a service route         | New need enters → OO creates route object, owner requirement, initial receipt |
| DS-2 | Assign correct owner         | Route requires care/logistics/group/admin → OO routes to qualified owner      |
| DS-3 | Detect overdue follow-up     | Follow-up window expires → OO emits Andon/blocker receipt                     |
| DS-4 | Prevent premature closure    | Owner tries close route without final evidence → OO refuses closure           |
| DS-5 | Escalate sensitive route     | Sensitive content detected → OO requires human review                         |
| DS-6 | Transfer ownership           | Route owner changes → OO records handoff and preserves chain                  |
| DS-7 | Show route status to steward | Steward dashboard lists open/pending/refused/verified routes                  |
| DS-8 | Close route with proof       | Final action completed → evidence attached → receipt verified → route closes  |

---

## Persona 8 — The Scientific / Infrastructure Strategist

### Profile
The Infrastructure Strategist works on large systems: climate, urban repair, logistics, infrastructure, public health, supply chain, disaster response, or large-scale R&D.

### Core anxiety
“Massive systems become dashboards, rhetoric, and unverifiable claims instead of measured interventions.”

### Desired outcome
Huge systems become representable as object-centric interventions with routes, measurements, receipts, refusals, and replay.

### Success condition
The strategist can model large-scale repair as many measurable micro-interventions with verifiable closure.

### 8 JTBD end-to-end tests

| ID   | Job                           | End-to-end proof                                                                          |
| ---- | ----------------------------- | ----------------------------------------------------------------------------------------- |
| IS-1 | Model intervention class      | User defines repair/intervention type → OO maps to public terms and evidence requirements |
| IS-2 | Admit local repair route      | Condition detected → OO opens route with object set, intervention type, measurement plan  |
| IS-3 | Require measurement evidence  | Intervention claims success without before/after evidence → OO refuses closure            |
| IS-4 | Bind sensor/process data      | Measurement uploaded → OO links sensor/object/event/receipt                               |
| IS-5 | Detect tampered measurement   | Evidence hash mismatch → OO emits tamper receipt                                          |
| IS-6 | Replay intervention history   | User asks “what happened here?” → OO reconstructs route timeline                          |
| IS-7 | Compare intervention outcomes | OO compares verified outcomes across sites and flags weak routes                          |
| IS-8 | Scale only verified pattern   | Proposed expansion requires verified receipts from prior interventions                    |

---

## Cross-Persona Closure Matrix

Each persona needs the same 8 proof dimensions, projected differently:

| Proof dimension   | Human meaning                               |
| ----------------- | ------------------------------------------- |
| Public alignment  | Does this belong to shared reality?         |
| Policy admission  | Is this action allowed?                     |
| Actuation         | Did the system actually do it?              |
| Positive evidence | Did valid work succeed?                     |
| Negative evidence | Did invalid work fail correctly?            |
| Receipt           | Can the claim be reconstructed?             |
| Projection        | Can the right person understand the state?  |
| Closure/refusal   | Is it done, blocked, pending, or escalated? |

---

> **Final Compression:**
> Each persona has a different front door. Each front door enters the same route:
> `intent → public alignment → policy admission → controlled actuation → receipt → verification → user-appropriate projection → closure or refusal`
