# Signing-key rotation (v26.9.24)

Recorded 2026-09-24 (fleet key scan after the single-repo migration). Base `17cd4c8aa365` of `open-ontologies`.
Every private key listed here was committed to this repository and is therefore compromised: every receipt or
attestation signed with it carries no signing authority (standing REFUSED, broken_term R_missing_authority).
The keys leave the tree (history is not rewritten; no force-push), and each key directory's `.gitignore` now
covers both halves. Every checkout keeps its own pair: ggen generates one on first use, and a tracked public
half without its private half would make that first `ggen sync` refuse [FM-KEY-010/011]. The canonical
checkout's new public key is published below for anyone verifying its future receipts.

| key dir | removed private key sha256 | removed public key sha256 | new public key (canonical checkout) |
|---|---|---|---|
| `.ggen/keys` | removed earlier | `299a26d40fb4b6c8e1d6ef389ffca3ea1e7d8ef58693b00df8a57d6a03c6cc67` | `99a0be249e16c34e2820954207dd2211ac77ee7d6b8a42e968fb1a4e31484a39` |

## Keys exposed on non-default branches (revoked 2026-09-24)

The v26.9.24 rotation scanned default branches only. A scan of every `origin/*` branch found the
private keys below committed on non-default branches only. Each is compromised and revoked: any receipt
or attestation signed with it carries no signing authority (standing REFUSED, broken_term
R_missing_authority). A disk scan of the canonical checkouts on 2026-09-24 found three of these keys in
use (ggen/packs, ignored files) and replaced them with fresh pairs. Copies in agent worktrees and tool
caches may still hold them. History is not rewritten, so the branches keep the blobs.

| path | private key sha256 | derived public key | branches (count, first) |
|---|---|---|---|
| `.ggen/keys/signing.key` | `84338837d09a11cfa18ec0d29ac080918b3ac232030650685c8994595dde4782` | `279b8a2c484a8b9c2be03398500afea33393c3f72c6ba467b2f646a9a1067c6c` | 29, `agent/a2a-main-entrypoint-path` |
