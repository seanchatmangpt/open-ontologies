# Compromised ggen receipt-signing keys (v26.9.23)

Recorded 2026-09-24 (operator security item, single-repo migration). These private ed25519 seeds were committed to this
PUBLIC repository's default branch, so they are compromised: every receipt signed with them carries NO signing authority
(standing of such signatures: REFUSED, broken_term R_missing_authority). This commit removes them from the tree (history is
not rewritten; no force-push) and ignores `.ggen/keys/signing.key`; the next `ggen` run generates a fresh, untracked
keypair (rotation). ggen is being fixed to write `.ggen/keys/.gitignore` itself so the class cannot recur.

| private key path (removed) | sha256 of the compromised private key file | sha256 of its public verifying.key |
|---|---|---|
| `.ggen/keys/signing.key` | `84338837d09a11cfa18ec0d29ac080918b3ac232030650685c8994595dde4782` | `299a26d40fb4b6c8e1d6ef389ffca3ea1e7d8ef58693b00df8a57d6a03c6cc67` |
