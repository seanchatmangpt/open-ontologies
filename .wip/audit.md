# WIP audit

Base: `0d553c77e86ae252ee627b5e11843a08f3b36ee5`
Audit branch: `bd2e835328737e1b46c679154a5af810eefdde67`

## Marker findings (280)

- `CHANGELOG.md:76` — `revoked_principals` table lands (TODO marker in code). New
- `CHANGELOG.md:323` — TODO(R6) — same disease, deferred fix template.
- `CHANGELOG.md:357` — `TODO(R6 §15.A9)`, `TODO(R6 §15.A11)`, `TODO(R6 §15.A12)` comments
- `CHANGELOG.md:678` — - `engine="groq_pm4py"` subprocess transport — MCP handlers now spawn the real DSPy/pm4py-backed translator instead of the in-process stub.
- `CHANGELOG.md:774` — - [`ee90af9`](../../commit/ee90af9) `fix(no-stub): wire ingest/map/extend/push format params; add named-graph push; remove dead cfg`
- `CHANGELOG.md:794` — - [`e4db225`](../../commit/e4db225) `ontostar(R3): wire PowlBridgeReplay into admission gate, replace stub`
- `CHANGELOG.md:804` — - `PowlBridgeReplay` replaces the noop stub in the admission gate.
- `DISSERTATION.md:46` — *   **L6 High-Speed Kernels:** Represented by `wasm4pm` stream-2 stub bindings and the `src/manufacturing/` pipeline.
- `IMPLEMENTATION_SUMMARY.md:142` — ## What's NOT Implemented (Vision 2030 Future Layers)
- `Cargo.toml:86` — todo = "deny"
- `ontology/alignment-notes.md:109` — `OcelObjectType` is a typing class for OCEL 2.0 instances (e.g. Order, Item, Package). In the current TTL it is a vocabulary stub — the actual object type individuals (Order, Item, etc.) are not defined. The enforcer correctly flags it as orphaned.
- `tests/end_to_end_governed_release.rs:8` — //!   5. assert Admitted(receipt) with non-stub fitness
- `tests/end_to_end_governed_release.rs:68` — //    fitness, not a stub 1.0).
- `tests/end_to_end_governed_release.rs:78` — !conf.run_id.starts_with("stub-run-"),
- `tests/end_to_end_governed_release.rs:79` — "verdict came from the stub, not the real PowlBridge: {}",
- `tests/ratchet_red_team.rs:43` — let msg = "evaluate_admission(stub) called";
- `tests/portability_codegen.rs:94` — let receipt = build_test_receipt(b"some-artifact-placeholder");
- `tests/saboteur_meta.rs:5` — //! output is structurally distinct from a hypothetical "stub that
- `tests/adversarial_jtbd_test.rs:114` — Either the format was silently ignored (soft stub) or auto-detection hid the override.\n\
- `tests/saboteur_a9_provenance_chain_load_bearing.rs:27` — //! R5 WB-1's TODO comment at admission.rs:660-662 carried A9 forward
- `tests/three_layer_integration_test.rs:2` — //! (#43 Dynamics → #44 Causal hookup → #45 Planner stub).
- `tests/three_layer_integration_test.rs:57` — // The Planner stub must read from the same SQLite store that
- `tests/manufacturing_validators.rs:191` — // 64-char WO hash literal with a placeholder of equal length.
- `tests/manufacturing_validators.rs:192` — let placeholder: String = "f".repeat(wo_hash.len());
- `tests/manufacturing_validators.rs:194` — f.contents = f.contents.replace(&wo_hash, &placeholder);
- `tests/ed25519_attestation.rs:3` — //! Replaces the digest-equality tautology stub with cryptographic
- `tests/ggen_revops_pipeline.rs:37` — /// the ggen pipeline produced a populated file rather than an empty stub.
- `tests/ggen_pipeline_real.rs:25` — /// 1. generated.rs exists and contains the expected stub modules.
- `tests/admission_real_replay.rs:2` — //! `PowlBridgeReplay`, not the `NoopPowlReplay` stub.
- `tests/admission_real_replay.rs:59` — // The bridge — not the stub — produced this verdict. The stub would
- `tests/admission_real_replay.rs:60` — // have returned the literal {fitness: 1.0, run_id: "stub-run-..."}.
- `tests/admission_real_replay.rs:62` — !conf.run_id.starts_with("stub-run-"),
- `tests/admission_real_replay.rs:63` — "verdict came from NoopPowlReplay stub, not PowlBridge: run_id={}",
- `packages/declarations.d.ts:29` — placeholder?: string;
- `src/server.rs:75` — /// (Stream-2 stub path; not backed by real POWL replay). 6 occurrences.
- `src/server.rs:1008` — // TODO(R3 Task B): replace tenant_id fallback with
- `src/server.rs:2551` — #[tool(name = "onto_policy_register", description = "Register an ARGOS-style policy rule (#40, ISWC 2025 WOP). `effect` is `\"allow\"` or `\"deny\"`; `condition` is a SPARQL ASK that can use the `{target}` placeholder. Pairs with `onto_policy_check` and `onto_certify_action` — CIVeX gates causal risk, ARGOS gates authorisation.")]
- `src/server.rs:2964` — #[tool(name = "onto_plan_classical", description = "Invoke Fast Downward as a subprocess on a precompiled PDDL domain + problem (#50). Returns the raw sas_plan content plus a parsed `operators` list (operator name + positional PDDL args). The orchestrator maps args back to original IRIs using the schema parameter names (still client-side per LLM-Modulo). If Fast Downward is not on PATH and `fast_downward_bin` is not set, returns a clean `binary_unavailable` error rather than falling back to a si
- `src/server.rs:2996` — #[tool(name = "onto_plan_compile_pddl", description = "Compile a PDDL domain from registered Dynamics action schemas (#43) plus a problem instance from the loaded graph and a goal Turtle slice (#45 Planner stub). Returns {domain, problem, translation_notes}. The actual planner (Fast Downward) is wrapped client-side per the LLM-Modulo convention — this primitive only emits the PDDL. Lossy in the v0.4 stub: only ASK-shape SPARQL preconditions translate cleanly; SELECT-shaped preconditions are pres
- `src/server.rs:5605` — // TODO(stream1-4): Streams 1-4 are not yet merged on this branch. The
- `src/server.rs:5859` — // TODO(stream1): replace with self.onto_declare_workflow(...).
- `src/server.rs:5981` — // Load scope row (Stream 5 stub schema).
- `src/server.rs:5986` — // delta — not a placeholder.
- `src/server.rs:6830` — // stub (NoopPowlReplay) or the real wasm4pm bridge.  The stub
- `src/server.rs:6831` — // prefixes every run_id with "stub-run-" so callers and auditors
- `src/server.rs:6832` — // can distinguish stub-path admissions from production-verified
- `src/server.rs:6836` — let powl_stub = receipt.record.conformance_run_id.starts_with("stub-run-");
- `src/server.rs:6844` — // integrated (stream-2 stub); CTQ admission skipped the real
- `src/server.rs:7709` — /// TODO(R3 Task B): replace `is_admin_principal` with the canonical
- `src/server.rs:7999` — /// TODO(R3 Task B): switch the INSERT target to `revoked_principals`
- `src/ocel_store.rs:478` — // Make sure the conformance_runs table exists. The Stream-3 stub
- `src/embed.rs:59` — /// let placeholder: Vec<f32> = vec![0.0; BGE_SMALL_DIM];
- `src/embed.rs:60` — /// assert_eq!(placeholder.len(), BGE_SMALL_DIM);
- `src/cell8.rs:17` — //! ## Phase 10 stub status
- `src/policy.rs:27` — /// SPARQL ASK. Can use the placeholder `{target}` which is substituted
- `src/attestation.rs:3` — //! Replaces the Phase-10 A10 tautology stub (which compared
- `src/cell_ready.rs:54` — /// Stream-2-stub stand-in for `wasm4pm`'s POWL arena handle. Stream 2
- `src/cell_ready.rs:221` — // ── Real Ed25519 attestation (replaces the A10 tautology stub) ──
- `src/plan_pddl.rs:1` — //! Planner stub (#45) — PDDL emission from Dynamics action schemas.
- `src/plan_pddl.rs:9` — //! ## Bounded scope (v0.4 stub)
- `src/plan_pddl.rs:24` — //! paper is the anchor for the rigorous version; this stub is a sand-table.
- `src/plan_pddl.rs:34` — /// stub couldn't fully encode and that was preserved as a PDDL comment.
- `src/plan_pddl.rs:75` — /// Translate a triple position (placeholder `<{x}>`, full IRI `<...>`, or bare
- `src/plan_pddl.rs:196` — // Problem stub — empty init/goal; the MCP layer fills in init from the
- `src/defects.rs:49` — /// (replaces the digest-equality tautology stub). Forward-compatible —
- `src/defects.rs:203` — /// matches the artifact bit-for-bit. (Phase-10 stub: digest-equality
- `src/state.rs:173` — -- ─── OntoStar Stream 1 stub migrations (authoritative copies live in Stream 1) ──
- `src/state.rs:252` — -- the §28 hidden-WIP leak. (Verified: no DELETE statement in
- `src/plan_classical.rs:25` — //! NOT fall back to a silent stub.
- `src/socket_windows.rs:5` — /// Windows stub for the Unix domain socket adapter.
- `src/admission.rs:5` — //!    (TODO Stream 2: replace [`PowlReplay`] stub with the real bridge),
- `src/admission.rs:225` — /// TODO(R3 Task B): switch from `revoked_sessions` fallback to the
- `src/admission.rs:525` — /// Stream-2-stub trait. The real implementation lives in `powl_bridge.rs`
- `src/admission.rs:529` — /// TODO(stream-2): swap to the wasm4pm-backed bridge. This stub MUST be
- `src/admission.rs:538` — /// flag distinguishes placeholder results from production-grade evidence.
- `src/admission.rs:569` — /// `true` when this result was produced by the stream-2 stub
- `src/admission.rs:576` — /// external operators and auditors can distinguish stub-path
- `src/admission.rs:595` — /// assert!(!pass.is_stub,             "pass() produces non-stub evidence");
- `src/admission.rs:623` — /// assert!(!fail.is_stub,             "fail() produces non-stub evidence");
- `src/admission.rs:684` — /// Returns `true` when this result was produced by the stream-2 stub
- `src/admission.rs:687` — /// Stub results have `fitness = 1.0` and `precision = 1.0` as placeholders
- `src/admission.rs:690` — /// # Auto-instinct: stub results produced by NoopPowlReplay always conform
- `src/admission.rs:698` — /// // Auto-instinct: stub results are always conformant (placeholder 1.0/1.0).
- `src/admission.rs:700` — /// assert!(stub_result.is_conformant(),  "stub result must report conformance");
- `src/admission.rs:702` — /// // A hand-crafted non-stub conforming result is not a stub.
- `src/admission.rs:711` — /// **Stream-2 stub.** Returns a perfect-fit verdict. Retained because some
- `src/admission.rs:724` — /// // Stub always reports perfect fitness and precision — these are
- `src/admission.rs:739` — /// Marker string placed in the codebase to locate every stream-2 stub site.
- `src/admission.rs:742` — /// remaining match indicates a stub that was not replaced with the real
- `src/admission.rs:750` — /// // The value is a recognisable TODO tag, not a semver.
- `src/admission.rs:751` — /// assert!(STREAM3_STUB_POWL_REPLAY_MARKER.starts_with("TODO"));
- `src/admission.rs:754` — pub const STREAM3_STUB_POWL_REPLAY_MARKER: &str = "TODO(stream-2): replace NoopPowlReplay";
- `src/admission.rs:833` — /// OCEL event attribute key indicating the POWL replay used a stub
- `src/admission.rs:913` — // POWL replay is not yet integrated (stream-2 stub);
- `src/admission.rs:916` — // `"stub-run-"` run_id prefix allow downstream consumers to
- `src/admission.rs:922` — run_id: format!("stub-run-{}", scope_token),
- `src/admission.rs:1427` — // Run conformance via wasm4pm bridge (or stub).
- `src/admission.rs:1462` — // self-attests with the same hash (placeholder until ed25519-dalek
- `src/admission.rs:2226` — // Run the stub migration on its own (it is a no-op SQL string but keep
- `src/admission.rs:2243` — // Surface stub status in the OCEL witness so a process miner who sees
- `src/admission.rs:2244` — // only the event log can flag stub-path runs without parsing run_id.
- `src/admission.rs:2287` — // placeholder values from NoopPowlReplay (stream-2 stub
- `src/admission.rs:2289` — // stub-path conformance runs as production evidence.
- `src/retention.rs:369` — // This is a placeholder for receipt-file artifacts — see
- `src/retention.rs:415` — /// implementation is a defined no-op placeholder — returns `0` until a
- `src/retention.rs:429` — /// // Placeholder implementation always reports zero orphans pruned.
- `src/receipts.rs:243` — /// stub migration in `STREAM3_STUB_MIGRATION`.
- `docs/01-architecture.md:55` — `PowlBridgeReplay` parses declared POWL strings via the `wasm4pm` crate, projects the OCEL trace tagged with `scope_token`, and returns a fitness/precision verdict. Production admission uses this; a `NoopPowlReplay` stub remains for gate-semantics unit tests that need a deterministic pass-through. Defects: `ReplayFailed`, `SkippedTask`, `ExtraTask`, `WrongOrder`, `CapabilityZero`, `ReplayDivergence`.
- `docs/05-receipt-chain.md:20` — pub signature: Option<Vec<u8>>,      // Ed25519 (Phase 10 stub-of-record)
- `docs/07-phase-history.md:11` — Commits `33feda7`, `e4db225`. Replaced the `NoopPowlReplay` stub with `PowlBridgeReplay::new(store)` in production-path admission. The gap closed: 12 admission tests had been validating against a fitness=1.0 stub. Production now routes through `wasm4pm` for every admission; only four gate-semantics tests retain the noop with an explicit `// INTENTIONAL` annotation.
- `docs/04-defect-taxonomy.md:38` — | `attestation_missing` | `AttestationMissing` | Cell8 A10 `ExternalAttestation` conjunct fails | Stub digest mismatch | `tests/cell8_thirteen_gates.rs` |
- `docs/ies-ecosystem.md:204` — | IES Top (ToLO) | ~22 | TBD | TBD | TBD | TBD | TBD |
- `docs/ies-ecosystem.md:205` — | IES Core | ~131 | TBD | TBD | TBD | TBD | TBD |
- `docs/00-overview.md:34` — - **Receipts prove.** Every admitted operation produces a `ProductionRecord`, a chained `Receipt` (BLAKE3 over canonical bytes, Ed25519-signed in Phase 10's stub-of-record form), and an `admission_granted` OCEL event. Denied operations produce `admission_denied` with a typed `defect` attribute. No claim of success exists outside this chain.
- `docs/00-overview.md:38` — The Phase 6 audit found 25 silently-broken CLI tests, 12 stub-validated admission tests, 21 dead defect variants, and 5 textual-ratchet bypass patterns. Phases 7–11 closed every finding fix-forward. The system now refuses to claim a feature works unless the receipt and the OCEL event log prove it ran. That refusal is the product.
- `.github/workflows/wip-sweep.yml:1` — name: WIP Sweep
- `.github/workflows/wip-sweep.yml:5` — branches: [agent/finish-wip-20260730]
- `.github/workflows/wip-sweep.yml:6` — paths: [.github/wip-trigger]
- `.github/workflows/wip-sweep.yml:18` — ref: agent/finish-wip-20260730
- `.github/workflows/wip-sweep.yml:21` — - name: Inventory WIP and executable gates
- `.github/workflows/wip-sweep.yml:26` — mkdir -p .wip
- `.github/workflows/wip-sweep.yml:33` — marker_re = re.compile(r'(?i)\b(TODO|FIXME|XXX|HACK|WIP|TBD|NOT[ _-]?IMPLEMENTED|PLACEHOLDER|STUB)\b|todo!\s*\(|unimplemented!\s*\(')
- `.github/workflows/wip-sweep.yml:109` — Path('.wip/audit.json').write_text(json.dumps(audit,indent=2))
- `.github/workflows/wip-sweep.yml:111` — lines=['# WIP audit','',f"Base: `{audit['base_sha']}`",'']
- `.github/workflows/wip-sweep.yml:128` — Path('.wip/audit.md').write_text('\n'.join(lines)+'\n')
- `.github/workflows/wip-sweep.yml:137` — git add .wip/audit.json .wip/audit.md
- `.github/workflows/wip-sweep.yml:138` — git commit -m 'chore: record WIP audit receipt'
- `.github/workflows/wip-sweep.yml:139` — git push origin HEAD:agent/finish-wip-20260730
- `ontology/zoela/core.ttl:181` — # § 3  Forward-declared stub classes for future modules
- `ontology/zoela/core.ttl:209` — # § 3b  Forward-declared stub classes for newly-created modules
- `case-studies/jc3iedm-ies4-crosswalk/README.md:39` — 4. **Use a placeholder JC3IEDM IRI namespace.** STANAG 5525 doesn't issue canonical web IRIs; this sketch uses `http://example.org/jc3iedm/` as a placeholder. A real implementation would use NATO/MIP-issued IRIs.
- `case-studies/jc3iedm-ies4-crosswalk/crosswalk.ttl:11` — # Placeholder JC3IEDM namespace — STANAG 5525 has no canonical web IRIs.
- `case-studies/mcp-ies-rag-demo/README.md:81` — - Uses a placeholder `demo:roleIn` helper property. A production IES4 implementation would use the canonical `ies:MemberState` + `ies:isMemberOf` pattern. The shortcut keeps the demo Turtle readable.
- `case-studies/zero-emission-aviation/demo/index.html:163` — <input id="search" type="search" autocomplete="off" placeholder="e.g. ZeroAvia, H2GEAR, electrolysis" aria-describedby="search-help" />
- `case-studies/verifiabench/results/multi_per_task_Qwen2.5-3B-Instruct-4bit.json:602` — "output": "```turtle\n<biolink:MTOR> <biolink:is_involved_in> <GO:0008150>\n```\n\nNote: The Gene Ontology (GO) term `GO:0008150` is not a real term for 'regulation of cell growth'. A correct representation would use a specific GO term for 'regulation of cell growth', but since one is not provided, I've used a placeholder here. If you have a specific GO term in mind, please provide it, and I can adjust the Turtle accordingly."
- `case-studies/verifiabench/results/multi_per_task_Qwen2.5-3B-Instruct-4bit.json:624` — "output": "```turtle\n@prefix bio: <http://biolink.info/vocabulary/> .\n<http://biolink.info/gene/ESR1> bio:involved_in bio:signal_transduction_process.\n<http://biolink.info/gene/ESR1> bio:is_a bio:Gene.\n<http://purl.obolinguistics.org/obo/GO_0008155> bio:is_a bio:Biological_process.\n```\n\nNote: The GO term `GO:0008155` is not a real Gene Ontology term. For the purpose of this example, I used `GO:0008155` as a placeholder for a hypothetical Gene Ontology term. The actual term to use would be
- `case-studies/skills-england-esco-crosswalk/pipeline/fetch_esco.py:48` — todo = [o for o in occs if o["stdCode"] not in done]
- `case-studies/skills-england-esco-crosswalk/pipeline/fetch_esco.py:49` — print(f"{len(todo)} to fetch ({len(done)} cached)")
- `case-studies/skills-england-esco-crosswalk/pipeline/fetch_esco.py:51` — for i, o in enumerate(todo):
- `case-studies/skills-england-esco-crosswalk/pipeline/fetch_esco.py:60` — print(f"{i}/{len(todo)}")
- `case-studies/heritage-aerial/pipeline/build-stac.py:138` — # An always-present placeholder for the primary image asset (the scan is
- `case-studies/heritage-aerial/demo/index.html:32` — #detail .placeholder { color: #999; font-style: italic; font-size: 0.9em; }
- `case-studies/heritage-aerial/demo/index.html:73` — <p class="placeholder">Click any marker on the map to view its NAPH metadata.</p>
- `case-studies/heritage-aerial/demo/index.html:191` — document.getElementById('detail').innerHTML = `<p class="placeholder">Could not load IIIF manifest: ${err.message}<br><br>Make sure you're serving this from a local HTTP server (not file://) and that <code>../reports/iiif-collection-manifest.json</code> exists.</p>`;
- `case-studies/heritage-aerial/demo/real.html:30` — #detail .placeholder { color: #999; font-style: italic; font-size: 0.9em; }
- `case-studies/heritage-aerial/demo/real.html:61` — <div id="detail"><p class="placeholder">Click any footprint to view its NAPH metadata.</p></div>
- `case-studies/heritage-aerial/demo/real.html:138` — load().catch(err=>{ document.getElementById('detail').innerHTML=`<p class="placeholder">Could not load manifest: ${err.message}. Serve over HTTP, not file://.</p>`; });
- `case-studies/heritage-aerial/docs/red-team-report.md:83` — **Status:** documented in the docs but not implemented. The CQ2 expected results table notes "(requires GeoSPARQL)" but doesn't say this means *not Oxigraph*.
- `case-studies/heritage-aerial/docs/red-team-report.md:124` — 2. Add a stub IIIF Image API in the demo pipeline that serves placeholder images so manifests fully resolve
- `case-studies/heritage-aerial/pipeline/scrapers/napl_opencanada.py:284` — for stub in stubs:
- `case-studies/heritage-aerial/pipeline/scrapers/napl_opencanada.py:286` — detail = dataset_detail(stub["id"])
- `case-studies/heritage-aerial/pipeline/scrapers/napl_opencanada.py:288` — print(f"# {stub['id']}: fetch failed: {e}", file=sys.stderr)
- `case-studies/heritage-aerial/pipeline/scrapers/__init__.py:9` — - NCAPAirPhotoFinderScraper — Angular SPA, requires Playwright/manual JSON capture (stub)
- `case-studies/heritage-aerial/pipeline/scrapers/__init__.py:10` — - USGSEarthExplorerScraper — M2M API, requires registration (stub)
- `case-studies/heritage-aerial/pipeline/scrapers/usgs_earthexplorer.py:3` — USGS Earth Explorer M2M API Adapter — STUB.
- `case-studies/heritage-aerial/pipeline/scrapers/usgs_earthexplorer.py:42` — description="USGS Earth Explorer NAPH adapter (STUB — requires USGS credentials)."
- `case-studies/heritage-aerial/pipeline/scrapers/usgs_earthexplorer.py:53` — print("# USGS Earth Explorer M2M adapter — STUB", file=sys.stderr)
- `case-studies/heritage-aerial/deliverables/06-knowledge-transfer/external-integrations.md:3` — For each NAPH gap previously marked "stub" or "skipped (external API)", this document maps to the existing open-source implementation worth adopting rather than rebuilding. Status as of 2026-05.
- `case-studies/heritage-aerial/deliverables/06-knowledge-transfer/external-integrations.md:32` — # See pipeline/scrapers/usgs_earthexplorer.py — fill in the TODO sections
- `case-studies/heritage-aerial/deliverables/06-knowledge-transfer/external-integrations.md:59` — 3. Update [`pipeline/iiif-bridge.py`](../../pipeline/iiif-bridge.py) so the `service.id` URLs point at your Cantaloupe instance instead of placeholder URLs
- `case-studies/heritage-aerial/deliverables/06-knowledge-transfer/external-integrations.md:144` — 2. Replace [`pipeline/scrapers/ncap_airphotofinder.py`](../../pipeline/scrapers/ncap_airphotofinder.py) stub with API client
- `case-studies/heritage-aerial/deliverables/06-knowledge-transfer/maintenance-runbook.md:33` — - Mailing list / forum: tbd
- `case-studies/heritage-aerial/deliverables/04-adoption-guidance/transition-guides/baseline-to-enhanced.md:57` — naph:flightAltitude 0.0 ;  # placeholder — flag as unknown
- `deploy/helm/values.yaml:58` — # If empty, the chart creates an empty placeholder Secret you must populate
- `packages/utils/crypto.ts:5` — // In production this delegates to a BLAKE3 WASM module; this stub provides
- `packages/utils/crypto.ts:9` — // Stub implementation — replace with wasm-blake3 or native binding in app workspace
- `packages/forms/zoela.tsx:28` — placeholder: "Integer priority for ordering push cards in the notification tray; lower values appear first.",
- `packages/forms/zoela.tsx:35` — placeholder: "Expo deep-link URI the card action button navigates to when tapped.",
- `packages/forms/zoela.tsx:42` — placeholder: "Label for the primary call-to-action button rendered on the push card (e.g. 'View Details', 'Accept Request').",
- `packages/forms/zoela.tsx:49` — placeholder: "Main body text of the push card, describing the action or update in detail.",
- `packages/forms/zoela.tsx:56` — placeholder: "Secondary line of text below the card title, providing route or ministry context.",
- `packages/forms/zoela.tsx:63` — placeholder: "Primary headline text displayed on the push card in the notification tray.",
- `packages/forms/zoela.tsx:84` — placeholder: "Expo deep-link URI that opens the relevant screen in ZOE LA Mobile when the notification is tapped.",
- `packages/forms/zoela.tsx:91` — placeholder: "Category identifier from the NotificationCategoryScheme, used to route and filter notifications in the app.",
- `packages/forms/zoela.tsx:98` — placeholder: "Full body text of the push notification providing context and call-to-action.",
- `packages/forms/zoela.tsx:105` — placeholder: "Short headline text of the push notification, displayed in the device notification shade.",
- `packages/forms/connectGroupInterestForm.tsx:38` — placeholder="e.g. Sunday evenings, weekday mornings"
- `packages/forms/connectGroupInterestForm.tsx:56` — placeholder="Anything else we should know?"
- `packages/screens/adminDetailScreen.tsx:21` — <Text style={styles.placeholder}>
- `packages/screens/adminDetailScreen.tsx:33` — placeholder: { fontSize: 14, color: '#666', fontStyle: 'italic' },
- `src/a2a/router.rs:43` — "task_id": "a2a-task-placeholder",
- `src/workflows/mod.rs:23` — //! confirming they are not placeholder stubs:
- `src/workflows/builtin.rs:87` — //   TODO(wasm4pm POWL v2): use CG=(...) once Choice Graphs land upstream
- `src/workflows/builtin.rs:133` — // CG{...} → XOR for now (TODO above).
- `src/workflows/builtin.rs:156` — // step (the alphabet still matches). TODO: revisit when POWL gains
- `src/cmds/generated.rs:20` — /// Generated stub — wired to src/cmds/doctor.rs
- `src/cmds/generated.rs:39` — /// Generated stub — wired to src/cmds/thesis.rs
- `src/cmds/generated.rs:58` — /// Generated stub — wired to src/cmds/marketplace.rs
- `src/cmds/generated.rs:77` — /// Generated stub — wired to src/cmds/clinical.rs
- `src/cmds/generated.rs:96` — /// Generated stub — wired to src/cmds/alignment.rs
- `src/cmds/generated.rs:115` — /// Generated stub — wired to src/cmds/governance.rs
- `src/cmds/generated.rs:134` — /// Generated stub — wired to src/cmds/data.rs
- `src/cmds/generated.rs:153` — /// Generated stub — wired to src/cmds/server.rs
- `src/cmds/generated.rs:172` — /// Generated stub — wired to src/cmds/ontology.rs
- `src/cmds/thesis.rs:250` — // Stub: return empty claim/evidence packets.
- `src/cmds/thesis.rs:261` — message: format!("Extraction stub for source_id={} — real LLM call TBD", source_id),
- `src/cmds/thesis.rs:293` — // Stub: run lightweight validation check
- `src/cmds/thesis.rs:307` — message: "Audit stub — SHACL validation TBD".to_string(),
- `src/cmds/thesis.rs:317` — // Stub: run chapter routing SPARQL CONSTRUCT
- `src/cmds/thesis.rs:329` — message: "Route stub — SPARQL CONSTRUCT TBD".to_string(),
- `src/cmds/thesis.rs:342` — // Stub: render markdown thesis
- `src/cmds/thesis.rs:370` — // Stub: run full pipeline
- `src/cmds/thesis.rs:416` — // Check 2: Thesis shapes file (stub: always pass)
- `src/cmds/thesis.rs:423` — // Check 3: Gemini connectivity (stub: assume pass)
- `src/cmds/thesis.rs:427` — "Gemini 3.1 Flash reachable (stub)".to_string(),
- `src/cmds/thesis.rs:444` — /// Stub: "Interactive flow TBD; see onto thesis wizard"
- `src/cmds/thesis.rs:449` — // Stub: interactive prompts TBD
- `src/cmds/thesis.rs:452` — "message": "Interactive wizard stub — prompts for input files, source metadata, chapter structure TBD"
- `src/cmds/thesis.rs:462` — // Stub: trace provenance
- `supabase/migrations/20260518000004_zoela_ocel_events.sql:21` — receipt_hash TEXT,                           -- SHA-256 hex digest (Web Crypto stub)
- `benchmark/oaei/README.md:39` — | **Open Ontologies** | **TBD** | **TBD** |
- `benchmark/reference/ies4.ttl:2516` — This is a very simple placeholder for an area of IES that is likely to grow in the future. For now, it can be used to group together a number of elements (using isPartOf relationship) to assert that they share the same truth - i.e. in one possible scenario, all of them were true. The same Element may exist in more than one PossibleWorld - i.e. scenarios may share elements. For version 4.1.0 of IES, PossibleWorld is to be used with AssessToBeTrue in order to specify a level of confidence or proba
- `benchmark/ontoaxiom/results/condition_d/goodrelations.json:16` — ["Product Or Services Some Instances Placeholder", "Product Or Service"],
- `benchmark/ontoaxiom/results/condition_d/goodrelations_extracted.json:16` — ["Product or services some instances placeholder (DEPRECATED)", "Product or service"],
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:9999` — "product or services some instances placeholder",
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:10114` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:10214` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:10310` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:10402` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:10490` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:10574` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:10654` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:10730` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:10802` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:10870` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:10934` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:10994` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:11050` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:11102` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:11150` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:11194` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:11234` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:11270` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:11302` — "product or services some instances placeholder"
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:11333` — "product or services some instances placeholder",
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:11337` — "product or services some instances placeholder",
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:11341` — "product or services some instances placeholder",
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:11345` — "product or services some instances placeholder",
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:11349` — "product or services some instances placeholder",
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:11353` — "product or services some instances placeholder",
- `benchmark/ontoaxiom/data/results/oo_bare_qwen_results.json:11357` — "product or services some instances placeholder",
- `benchmark/ontoaxiom/data/ontoaxiom/ontologies/goodrelations.ttl:205` — rdfs:comment "A placeholder for all official public holidays at the gr:Location. This allows specifying the opening hours on public holidays. If a given day is a public holiday, this specification supersedes the opening hours for the respective day of the week."@en ;
- `benchmark/ontoaxiom/data/ontoaxiom/ontologies/goodrelations.ttl:622` — rdfs:comment """This states that an actual product instance (gr:Individual) or a placeholder instance for multiple, unidentified such instances (gr:SomeItems) is one occurence of a particular gr:ProductOrServiceModel.
- `benchmark/ontoaxiom/data/ontoaxiom/ontologies/goodrelations.ttl:1060` — rdfs:label "Product or services some instances placeholder (DEPRECATED)"@en ;
- `benchmark/ontoaxiom/data/ontoaxiom/ontologies/goodrelations.ttl:1434` — rdfs:comment """A placeholder instance for unknown instances of a mass-produced commodity. This is used as a computationally cheap work-around for such instances that are not individually exposed on the Web but just stated to exist (i.e., which are existentially quantified).
- `benchmark/ontoaxiom/data/ontoaxiom/ontologies/goodrelations.ttl:1859` — rdfs:comment """The superclass of all classes describing products or services types, either by nature or purpose. Examples for such subclasses are "TV set", "vacuum cleaner", etc. An instance of this class can be either an actual product or service (gr:Individual), a placeholder instance for unknown instances of a mass-produced commodity (gr:SomeItems), or a model / prototype specification (gr:ProductOrServiceModel). When in doubt, use gr:SomeItems.
- `benchmark/ontoaxiom/data/ontoaxiom/ontologies/goodrelations.ttl:1864` — c) dummyCellPhone123 as a placeholder for actual instances of a certain kind of cell phones (gr:SomeItems)
- `benchmark/ontoaxiom/data/ontoaxiom/ontologies/time.ttl:434` — This is a stub class, representing the set of all temporal reference systems."""@en ;
- `benchmark/ontoaxiom/data/ontoaxiom/ontologies/time.ttl:438` — This is a stub class, representing the set of all temporal reference systems."""@en ;
- `benchmark/ontoaxiom/data/ontoaxiom/ontologies/time.ttl:547` — skos:note "An ontology for time zone descriptions was described in [owl-time-20060927] and provided as RDF in a separate namespace tzont:. However, that ontology was incomplete in scope, and the example datasets were selective. Furthermore, since the use of a class from an external ontology as the range of an ObjectProperty in OWL-Time creates a dependency, reference to the time zone class has been replaced with the 'stub' class in the normative part of this version of OWL-Time."@en ;
- `benchmark/ontoaxiom/data/ontoaxiom/classes/goodrelations_classes.json:29` — "product or services some instances placeholder",
- `benchmark/ontoaxiom/data/ontoaxiom/subclassof/goodrelations_subclassof.json:55` — "Product Or Services Some Instances Placeholder",
- `studio/src/components/PropertyInspector.tsx:292` — placeholder="rdfs:label or full URI..."
- `studio/src/components/PropertyInspector.tsx:339` — placeholder={newValType === 'uri' ? 'http://...' : 'value...'}
- `studio/src/components/TreeView.tsx:511` — <input type="text" placeholder="Search nodes..." value={searchTerm} onChange={e => setSearchTerm(e.target.value)}
- `studio/src/components/ChatPanel.tsx:147` — placeholder="Ask about ontologies..."
- `studio/src/components/AddClassDialog.tsx:38` — placeholder="Class name..."
- `studio/docs/plans/2026-03-14-tauri-implementation-plan.md:155` — {/* Graph canvas placeholder */}
- `studio/docs/plans/2026-03-14-tauri-implementation-plan.md:213` — Expected: Dark-themed window with toolbar, graph placeholder, chat panel, status bar
- `studio/docs/plans/2026-03-14-tauri-implementation-plan.md:924` — Replace graph placeholder div with:
- `studio/docs/plans/2026-03-14-tauri-implementation-plan.md:933` — // Replace graph placeholder:
- `studio/docs/plans/2026-03-14-tauri-implementation-plan.md:990` — placeholder="Class name..."
- `studio/docs/plans/2026-03-14-tauri-implementation-plan.md:1901` — placeholder="Ask about ontologies..."
- `studio/docs/plans/2026-03-14-tauri-implementation-plan.md:1963` — Replace the chat placeholder in Layout.tsx:
- `studio/docs/plans/2026-03-14-tauri-implementation-plan.md:1968` — // Replace the chat panel placeholder div content:
- `studio/docs/plans/2026-03-14-tauri-implementation-plan.md:2131` — // Replace inspector placeholder:
- `docs/plans/2026-03-09-standalone-mcp-server-plan.md:86` — **Step 2: Create stub lib.rs so it compiles**
- `docs/plans/2026-03-12-infrastructure-positioning-plan.md:262` — Add a shared setup helper and stub the `match` arms (just print `{"error":"not implemented"}` for now — we'll fill them in Tasks 2–6):
- `docs/plans/2026-03-12-infrastructure-positioning-plan.md:369` — Expected: FAIL — subcommands return `not implemented`.
- `docs/plans/2026-03-12-infrastructure-positioning-plan.md:1571` — Add new concise benchmark section with the HermiT/Pellet/OO comparison table (numbers TBD after running). Keep existing detailed benchmarks below as "Detailed Benchmark Methodology".
- `docs/research/PHD_PROGRAM.md:37` — *   *Lab:* Executing the `wasm4pm` stream-2 stub bindings via the `src/swarm.rs` cognition swarm.
- `docs/research/CODEBASE_MAPPING.md:11` — *   **L6 High-Speed Kernels:** Represented by the `wasm4pm` stream-2 stub bindings (POWL and cognition kernels) and the `src/manufacturing/` pipeline that targets AtomVM, Erlang, and Rust.
- `docs/research/SYLLABI.md:114` — *   **Weeks 10-12: The `wasm4pm` Lab:** Integrating and evaluating the stream-2 stub bindings. Profiling kernel execution speed.
- `docs/research/materials/EDGE_LAB_SPEC.md:14` — *   **Wasm Runtime:** WasmEdge runtime configured with the `open-ontologies` `wasm4pm` stream-2 stub bindings.
- `.specify/templates/zoela/connect-group-interest-form.tera:38` — placeholder="e.g. Sunday evenings, weekday mornings"
- `.specify/templates/zoela/connect-group-interest-form.tera:56` — placeholder="Anything else we should know?"
- `.specify/templates/zoela/react-hook-form.tera:69` — placeholder: "{{ fcomment | trim }}",
- `.specify/templates/zoela/admin-detail-screen.tera:42` — <Text style={styles.placeholder}>
- `.specify/templates/zoela/admin-detail-screen.tera:54` — placeholder: { fontSize: 14, color: '#666', fontStyle: 'italic' },
- `.specify/templates/cli/cmds.rs.tera:49` — /// Generated stub — wired to src/cmds/{{ name | trim | replace(from="-", to="_") }}.rs

## Zero-byte files (0)


## Non-main branches (3)

- `origin/ontostar-integration` — ahead 72, behind 44, `ec2115642aa1` — docs: finalize final_proof.txt
- `origin/agent/finish-wip-20260730` — ahead 3, behind 3, `bd2e83532873` — chore: retrigger WIP sweep
- `origin/agent/chatmangpt-namespace-26.7.29` — ahead 1, behind 95, `3a5defe909a9` — chore(namespace): bind open-ontologies to chatmangpt.com

## Duplicate Cargo.lock packages (3)

- `('oxrdf', '0.3.3', 'registry+https://github.com/rust-lang/crates.io-index')` blocks 309 and 310
- `('oxttl', '0.2.3', 'registry+https://github.com/rust-lang/crates.io-index')` blocks 315 and 316
- `('wit-bindgen', '0.57.1', 'registry+https://github.com/rust-lang/crates.io-index')` blocks 606 and 607

## Executable gates

### `cargo metadata --locked --format-version 1 --no-deps`
Exit: `0` in 1.16s
```text
home/runner/work/open-ontologies/open-ontologies/tests/shacl_shared_receipt.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"shacl_test","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/shacl_test.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"shaped_translator_e2e","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/shaped_translator_e2e.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"socket_test","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/socket_test.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"solution_manufacturing_e2e","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/solution_manufacturing_e2e.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"sql_ingest_handler","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/sql_ingest_handler.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"sqlsource_test","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/sqlsource_test.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"state_v2_test","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/state_v2_test.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"structembed_test","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/structembed_test.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"tableaux_test","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/tableaux_test.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"tenant_isolation_audit","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/tenant_isolation_audit.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"terraform_loop_test","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/terraform_loop_test.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"three_layer_integration_test","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/three_layer_integration_test.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"threshold_real","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/threshold_real.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"toolfilter_test","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/toolfilter_test.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"vecstore_test","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/vecstore_test.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"wb1_subprocess_timeout","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/wb1_subprocess_timeout.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"workflow_discover_real","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/workflow_discover_real.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["test"],"crate_types":["bin"],"name":"zoela_route_conformance","src_path":"/home/runner/work/open-ontologies/open-ontologies/tests/zoela_route_conformance.rs","edition":"2024","doc":false,"doctest":false,"test":true},{"kind":["bench"],"crate_types":["bin"],"name":"admission_bench","src_path":"/home/runner/work/open-ontologies/open-ontologies/benches/admission_bench.rs","edition":"2024","doc":false,"doctest":false,"test":false},{"kind":["bench"],"crate_types":["bin"],"name":"manufacturing_bench","src_path":"/home/runner/work/open-ontologies/open-ontologies/benches/manufacturing_bench.rs","edition":"2024","doc":false,"doctest":false,"test":false},{"kind":["bench"],"crate_types":["bin"],"name":"receipts_bench","src_path":"/home/runner/work/open-ontologies/open-ontologies/benches/receipts_bench.rs","edition":"2024","doc":false,"doctest":false,"test":false},{"kind":["bench"],"crate_types":["bin"],"name":"swarm_bench","src_path":"/home/runner/work/open-ontologies/open-ontologies/benches/swarm_bench.rs","edition":"2024","doc":false,"doctest":false,"test":false}],"features":{"bincode":["dep:bincode"],"default":[],"duckdb":["dep:duckdb"],"embeddings":["tract-onnx","tokenizers"],"instant-distance":["dep:instant-distance"],"mcpp":["dep:mcpp-core"],"postgres":["sqlx"],"sql":["postgres","duckdb"],"sqlx":["dep:sqlx"],"tokenizers":["dep:tokenizers"],"tract-onnx":["dep:tract-onnx"]},"manifest_path":"/home/runner/work/open-ontologies/open-ontologies/Cargo.toml","metadata":null,"publish":null,"authors":[],"categories":[],"keywords":[],"readme":"README.md","repository":"https://github.com/fabio-rovai/open-ontologies","homepage":null,"documentation":null,"edition":"2024","links":null,"default_run":null,"rust_version":null}],"workspace_members":["path+file:///home/runner/work/open-ontologies/open-ontologies#26.5.13"],"workspace_default_members":["path+file:///home/runner/work/open-ontologies/open-ontologies#26.5.13"],"resolve":null,"target_directory":"/home/runner/work/open-ontologies/open-ontologies/target","build_directory":"/home/runner/work/open-ontologies/open-ontologies/target","version":1,"workspace_root":"/home/runner/work/open-ontologies/open-ontologies","metadata":null}


```
### `cargo check --locked --all-targets`
Exit: `101` in 0.61s
```text

error: failed to parse lock file at: /home/runner/work/open-ontologies/open-ontologies/Cargo.lock

Caused by:
  package `oxrdf` is specified twice in the lockfile

```
