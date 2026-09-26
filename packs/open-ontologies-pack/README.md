# open-ontologies ggen pack

This is the repository-local, self-hosted ggen pack for the canonical open-ontologies CLI surface.

Authority remains outside the pack: `ontology/cli-open-ontologies.ttl` is the semantic source of truth, while `.specify/queries/cli/commands_aggregated.rq` and `.specify/templates/cli/cmds.rs.tera` are the canonical manufacturing inputs. Their counterparts in this directory are deterministic byte-for-byte projections so the pack can be consumed as a closed directory without creating a second semantic authority.

Manufacture/replay from repository root:

```sh
python3 tools/ggen_pack.py --write
python3 tools/ggen_pack.py --check
```

Generate from the pack directory with ggen:

```sh
cd packs/open-ontologies-pack
ggen sync run
ggen receipt verify
```

The executable repository verifier pins the ggen implementation SHA and its Rust toolchain, performs a dry run, two real sync runs, verifies both receipts, and refuses nondeterministic generated output:

```sh
bash tools/verify_ggen_pack_runtime.sh
```

`generated/cmds.rs` is a pack-local projection and is not a hand-editing surface. A mismatch between root authorities and pack projections is `REFUSED:GGEN_PACK_DRIFT`; divergent replay output is `REFUSED:GGEN_PACK_NONDETERMINISTIC`.
