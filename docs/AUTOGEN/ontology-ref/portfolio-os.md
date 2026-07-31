# portfolio-os.ttl

_Generated 2026-05-18T19:57:13Z by `tools/repo-state/generators/ontologies-ttl-to-md.py`._

- **Source:** `ontology/portfolio-os.ttl`
- **Triples:** 270
- **Classes:** 8 · **Properties:** 44 · **SHACL shapes:** 0

## Classes

| name | label | comment |
|---|---|---|
| `Andon` | Portfolio Andon | An alert or exception signal. Blocks portfolio progress until resolved (recovered or signed off by Sean). |
| `Cell` | Portfolio Cell | A repository or autonomous work unit in the portfolio. Each cell has defined boundaries, obligation queue, receipt chain |
| `Convergence` | Portfolio Convergence | A portfolio-level observation: snapshot of cell states, receipt chain head, active andons, and queue depth. |
| `Obligation` | Portfolio Obligation | A unit of work in the portfolio queue. Links an action, a cell, and mandatory/optional completeness requirements. |
| `ObligationQueue` | Obligation Queue | The set of active obligations in the portfolio, partitioned by state: pending, in-flight, done, blocked, needs-human. |
| `Receipt` | Portfolio Receipt | Proof of execution: BLAKE3-signed chain entry recording a tick outcome, cell action, or obligation resolution. |
| `ReceiptChain` | Receipt Chain | The append-only chain of all portfolio receipts. Immutable; chain integrity is the source of truth for authorization. |
| `Tick` | Portfolio Tick | A deterministic execution cycle in the portfolio. One tick per invocation of the portfolio kernel. |

## Properties

| name | domain | range | comment |
|---|---|---|---|
| `andonAutoCleared` | Andon | boolean | True if the andon was automatically cleared (e.g., STALE_LOCK cleared after time |
| `andonCell` | Andon | Cell | The cell that triggered the andon. |
| `andonCode` | Andon | string | Andon type code (e.g., STALE_LOCK, DEFECT_DETECTED, HUMAN_DECISION_NEEDED). |
| `andonMessage` | Andon | string | Human-readable description of the andon condition. |
| `andonRecoveryReceipt` | Andon | Receipt | The receipt documenting recovery from this andon. Required even for auto-clearab |
| `andonSeverity` | Andon | string | Severity level: red (critical, blocks), orange (warning), yellow (info). |
| `cellAgent` | Cell | string | Assigned agent (Claude, Gemini, human name) for this cell's work orders. |
| `cellId` | Cell | string | Unique identifier for the cell (repository name or cell code). |
| `cellRepo` | Cell | string | Git repository path or URL for the cell. |
| `cellStatus` | Cell | string | Current status: active, frozen, quarantined, retired. |
| `chainHead` | ReceiptChain | Receipt | The most recent receipt in the chain. |
| `chainIntegrity` | ReceiptChain | boolean | True if all receipts in the chain verify (hash linkage and signatures). |
| `convergenceActiveAndons` | Convergence | integer | Number of unresolved andons blocking portfolio progress. |
| `convergenceActiveCells` | Convergence | integer | Number of active (non-frozen, non-retired) cells. |
| `convergenceCells` | Convergence | integer | Number of cells in the portfolio at this observation. |
| `convergenceQueueDepth` | Convergence | integer | Total obligations in the queue (pending + in-flight + needs-human). |
| `convergenceQueueInflight` | Convergence | integer | Obligations currently being executed. |
| `convergenceQueueNeedsHuman` | Convergence | integer | Obligations awaiting human (Sean) decision or signature. |
| `convergenceQueuePending` | Convergence | integer | Obligations in pending state (waiting to start). |
| `convergenceReceiptChainHead` | Convergence | Receipt | The most recent receipt in the chain. |
| `convergenceTimestamp` | Convergence | dateTime | Timestamp of this convergence observation. |
| `obligationAction` | Obligation | string | What the obligation requires: implement, review, sign, dispatch, etc. |
| `obligationAssignee` | Obligation | string | Assigned agent or person responsible for the obligation. |
| `obligationCell` | Obligation | Cell | The cell that must execute the obligation. |
| `obligationHasPermission` | Obligation | Permission | Explicit permissions defining what the assignee may do. |
| `obligationHasProhibition` | Obligation | Prohibition | Explicit prohibitions defining what the assignee may not do. |
| `obligationId` | Obligation | string | Unique obligation identifier. |
| `obligationStatus` | Obligation | string | Current status: pending, in-flight, done, blocked, needs-human. |
| `queueInflightCount` | ObligationQueue | integer | Number of in-flight obligations. |
| `queueNeedsHumanCount` | ObligationQueue | integer | Number of obligations awaiting human action. |
| `queuePendingCount` | ObligationQueue | integer | Number of pending obligations. |
| `queueState` | ObligationQueue | Obligation | An obligation in this queue. |
| `receiptContent` | Receipt | string | JSON or canonical serialization of the receipt data. |
| `receiptHash` | Receipt | string | BLAKE3 hash of the receipt content. |
| `receiptId` | Receipt | string | Unique receipt identifier (hash or sequential number). |
| `receiptPrevHash` | Receipt | string | BLAKE3 hash of the previous receipt in the chain. Links to parent. |
| `receiptSignature` | Receipt | string | Ed25519 signature of the receipt hash (signed by portfolio signer, e.g., Sean). |
| `receiptTimestamp` | Receipt | dateTime | ISO 8601 timestamp when the receipt was emitted. |
| `tickCell` | Tick | Cell | The cell acted upon in this tick. |
| `tickEndTime` | Tick | dateTime | Timestamp when the tick completed. |
| `tickId` | Tick | string | Sequential tick identifier (e.g., tick-000050). |
| `tickPhase` | Tick | string | DFLSS phase: Define, Measure, Analyze, Design, Improve, Verify. |
| `tickStartTime` | Tick | dateTime | Timestamp when the tick started. |
| `wasGeneratedByTick` | Receipt | Tick | The tick that generated this receipt. |
