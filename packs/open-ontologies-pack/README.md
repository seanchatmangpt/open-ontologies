# open-ontologies ggen pack

This is the repository-local, self-hosted ggen pack for the canonical open-ontologies CLI surface.

Authority remains outside the pack: `ontology/cli-open-ontologies.ttl` is the semantic source of truth, while `.specify/queries/cli/commands_aggregated.rq` and `.specify/templates/cli/cmds.rs.tera` are the canonical manufacturing inputs. Their counterparts in this directory are deterministic byte-for-byte projections so the pack can be consumed as a closed directory without creating a second semantic authority.

Manufacture/replay from repository root:

```sh
python3 tools/ggen_pack.py --write
python3 tools/ggen_pack.py --check
```

Generate from the pack directory with the repository-pinned ggen toolchain:

```sh
cd packs/open-ontologies-pack
ggen sync
```

`generated/cmds.rs` is a pack-local projection and is not a hand-editing surface. A mismatch between root authorities and pack projections is `REFUSED:GGEN_PACK_DRIFT`.
