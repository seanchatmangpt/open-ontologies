---
name: open-ontologies Absolute Rules
description: 7 non-negotiable rules for RDF/OWL ontology engineering
type: rules
---

# Absolute Rules (Always-Loaded)

## 1. NEVER Edit `src/cmds/generated.rs` Directly

- `src/cmds/generated.rs` is a **ggen artifact** — automatically generated from `ontology/cli-open-ontologies.ttl`
- Edits to `generated.rs` will be **lost** on next `ggen sync`

**To change CLI surface:**
1. Edit `ontology/cli-open-ontologies.ttl` (the RDF source)
2. Run `ggen sync`
3. `src/cmds/generated.rs` is regenerated automatically

## 2. NEVER Edit Cell8 TTL Without Running SHACL Validation After

If you edit `ontology/cell8-*.ttl`:
1. Make your changes
2. Run: `onto validate ontology/cell8-shapes.ttl`
3. Verify: all validation passes (exit 0)

**If validation fails:**
- Fix the TTL and re-validate
- Do NOT proceed until validation passes

**Why:** Cell8 gates are conformance requirements. Invalid SHACL shapes break Gate A1-A13.

## 3. ALL `onto_*` Tools Must Be Registered in `tool_router!`

When adding a new tool to the MCP server:

1. Implement the tool function in `src/cmds/`
2. Add to `tool_router!` macro in `src/server.rs`:
   ```rust
   tool_router! {
       // ...
       "onto_new_tool" => handle_new_tool(ctx, params),
   }
   ```

**If not registered:** Tool will not be available via MCP.

## 4. `let _ = param;` is FORBIDDEN — dead-param-gate Blocks It

- FORBIDDEN: `let _ = unused_param;` (ignoring parameters)
- FORBIDDEN: `#[allow(unused_variables)]` silencing the warning

**If you have an unused parameter:**
1. Remove it from the function signature
2. Update all callers
3. Or: use it for something

**Why:** `let _ = param;` is "theater code" — pretending to use it without actually doing so. The dead-param gate (`tools/dead-param-gate.sh`) runs on every make and will FAIL if these patterns exist.

## 5. `make adversarial` Must Pass Before Claiming Completion

Before saying "done":

```bash
make adversarial
```

This runs:
- Dead-param gate (no `let _ = param;`)
- Clippy deny list (`todo!`, `unimplemented!`, `dbg_macro`)
- Adversarial JTBD tests
- Full test suite

If `make adversarial` fails, you are **not done**. Fix and re-run until exit 0.

## 6. ALWAYS Use `make` — Never Direct `cargo` Alone

- FORBIDDEN: `cargo check`
- FORBIDDEN: `cargo build`
- FORBIDDEN: `cargo test`
- REQUIRED: `make check`, `make build`, `make test`

**Why:** The Makefile wraps cargo with SHACL validation, dead-param gates, and lineage recording. Direct cargo bypasses these.

## 7. STOP THE LINE on Andon Signals

Immediately halt and fix:

- `error[E` — compiler error
- `test.*FAILED` — test failure
- `dead param` — unused parameter pattern
- `panicked at` — runtime panic

**Do not proceed** until the signal clears.

---

**The strongest rule:** If the code compiles but SHACL validation fails, the code is broken — fix before merging.
