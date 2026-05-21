# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [26.5.21] - 2026-05-21

### Changed
- **Architecture Recenter**: Established `OpenOntologyReceipt.v1` as the primary truth surface. GHF, wasm4pm, ggen MCP/A2A, and ZOELA are now firmly structured as downstream route families bound by the same OCEL path grammar.
- **Receipt Validation**: Implemented core validation laws requiring explicit `expected_ocel`, `observed_ocel`, alignment state, and boundary evidence. Refuses closure for synthetic/cloned traces or exit-code-only proofs.
- **GHF Fleet Sentinel**: Demonstrated the first regression-bound route family with a complete refusal/remediation/admission loop (missing artifact -> OutOfMembraneReceipt -> ggen remediation -> FleetHealthReceipt).
- **AutoReceipt Manifest**: Formalized that `AutoReceiptClosed` is a derived theorem from the Validation Ladder, not a self-certified agent state.