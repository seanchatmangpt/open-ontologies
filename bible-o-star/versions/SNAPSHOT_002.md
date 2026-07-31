# SNAPSHOT_002 — Bible O* Rollback Manifest

**Checkpoint:** BIBLE_O_STAR_002
**Snapshot version:** 002
**Created:** 2026-06-02T00:00:00Z
**File count:** 52
**Hash algorithm:** BLAKE3 (b3sum)

## Purpose

This manifest records the BLAKE3 content hashes of all 52 files present in the
`bible-o-star` package at the BIBLE_O_STAR_002 checkpoint. To roll back to this
state, restore all files listed below from their recorded hashes.

## Rollback procedure

1. Verify the hash of every target file against this manifest.
2. For any file whose hash does not match, restore from the version-control
   object referenced by that hash.
3. Re-run `scripts/validate_bible_o_star.sh` to confirm conformance.
4. Emit a new receipt documenting the rollback event.

## File manifest (BLAKE3)

```
57c8f4a8117be51c5aaea4c228be002d65814ad1dc1f2d4345f3b21de67c44e9  ./AGENT_REPORTS/AGENT_1_REPORT.md
8b9444f29de62e3a8d670d2b09c0863fc5dfeae30b2084bc6f40dda1891adaeb  ./AGENT_REPORTS/AGENT_2_REPORT.md
0b65a542cc2a0496f0a1ad6a9d357d59d58c24336c12488f95030c7b7f2f0e65  ./AGENT_REPORTS/AGENT_3_REPORT.md
55dcfbd9268edb829e07ea546959d48de16a5b1dd5e450b0f5ef7d217148ddb7  ./AGENT_REPORTS/AGENT_4_REPORT.md
f9ed6ebc2a21666790f335bc131b3c4acd3630ff87170b74ab299360387da96f  ./BIBLE_O_STAR_001.md
9178b199ce5ad0941283c219d1f1814fdcf7a8bc330501d36c11d4a5061554be  ./BIBLE_O_STAR_002.md
9fa8b6087c63eecb27fe6987d9be1e8fb1a5b151c9e52c751ee0e0d4d895c141  ./docs/BUILDER_REGISTRY.md
d38ddc9dc984c0f86988e52304b7f863785570052b0c3d721345ceafdea547f8  ./docs/COURIER_FALSE_REPORT_MODEL.md
6ce18ab9c7a75a925ef111991681cba4c684f408a9e9ff1cfd3e57487635d9ca  ./docs/GATE_ASSIGNMENT_MODEL.md
dc76ae4f7ed335dade832da377d3d365fb85d1339685f2a2c439d97a3d5fc7d9  ./docs/GGEN_PIPELINE_NOTES.md
1c91b3ab114ab8aeacd807be02baf12499ba3449f1083b25820f9021ad1bcef8  ./docs/IP_AUDIT_RESULT.md
154ac1c274536d63dfe73d6321aab8764be5e23619a9ba538c7089be7b1a427f  ./docs/LICENSE_AND_USAGE_BOUNDARY.md
1095926666a08ad94ccbd335b7caceb0a38510916a25c48e42e78cb731f4e0a7  ./docs/MOCKERS_ADVERSARIAL_FEEDBACK.md
29d58c718b13185a4e0ee09ebda97c1f8a70f4417c7a299a3d357e12d3a592ba  ./docs/MUSTER_LEDGER.md
d0529b41507dbf0262919d938df271c3d8cfd3db3f4da460e2114e6d1b3c8434  ./docs/NATIONS_LEDGER.md
c4da18ed5f74d2ea281c1dc4376c0ab89cc7297870fb9e7f2b840a52005492f5  ./docs/NEHEMIAH_52_OPERATING_GRAMMAR.md
06cfb36a5577e6b2bb9df1f1a28cbba01d03e41d45a415fd6d30c17d3b25c1a5  ./docs/PRAYER_LAYER.md
21dd1d9cc0f30c2297a42a049a936a544a2e2eec8f39b910384b52db2e0dfd67  ./docs/PROPHETIC_PROCLAMATION_MODEL.md
1f2e5edd3270bd72a17ce6fe00612d59e51d28b24f728c7f1f801de50206ec11  ./docs/PUBLIC_SOURCE_LEDGER.md
84588198cf131e1acfb66deaf5bc69a208f3b305820c581dd5028aad1180ea6e  ./docs/TOGAF_MAPPING.md
e515e0ccba3f43681a9189f358e77815ff47f16dadab1bb4aa9a923acb64373a  ./docs/USURY_LEDGER.md
28459c15beb722bacd552d7592b2e86becf68bcdfc9f50a20f6f559db0bb3c5d  ./docs/WALL_SECTION_REGISTRY.md
d4fe45a3ae97b4d5d1fe4cf289f00c268c8a4350783d4d7050e3bcd20c2aa46a  ./examples/courier-false-report-record.ttl
ec63bc24fff3208fda4b1b3d35a11611400720be9fc848b72a8fa8843f00bfea  ./examples/dung-gate-record.ttl
87919b137dc7192773762d8855ee4f808471e8f56c373c38bc66c54b01584a4a  ./examples/east-gate-record.ttl
aedf57758a76fba3e46378a5ef0d584d47d4e157865969fef3dbff44cd309748  ./examples/fish-gate-landing-page.ttl
316970daef80aba2c549b2c5d1d912df35db6f58c12b2b6f0e718d06b188d3ef  ./examples/fountain-gate-record.ttl
52d9325308872ee55cb3913ad21bdf5212b71ee3f1c703fde5a14fc4ae290f3b  ./examples/horse-gate-record.ttl
00eb1abcc359a05b125b3c6ffff4d420e91e898779e18f1fde4b36efdd69ff3f  ./examples/inspection-gate-receipt.ttl
a71cc97b33f685892cd2fc670987f17f5416c3b489c3354d707c6cb5b323f352  ./examples/mocker-feedback-record.ttl
bd4951c5b9968860a16852ec546059c835c556ff106ecbe291fc813fef9300cc  ./examples/muster-ledger-record.ttl
dbb6e0a46093ffa692bef07a7578b1a22841af100cee2bd64f2ae3a1b5c0f313  ./examples/old-gate-record.ttl
ad7fcfe4282538b6a94e7e77207d64d41f4798f0d7b1ab7ad8aa3acc41bb3a6a  ./examples/sheep-gate-record.ttl
4f6b1e99de8f0f521b66380c996d3c89c478305334044b14abcf652d124c71f8  ./examples/usury-ledger-record.ttl
4037bd485e317e75eab3b431bcec3da4c4f9e4a08d305e0bc2ad559c2cd0175c  ./examples/valley-gate-record.ttl
2996bf805b5ed02ed8ea87d49f5b02aeb31aa7f455b8a2f8fb6de724584ed455  ./examples/water-gate-pericope.ttl
4ab618d60939e98ef75e3d3d8df17dac76e887efe1ebe0924cda786abac4cdd2  ./examples/water-gate-record.ttl
0a4211035f19a4415403d56d9099877b7fce0e817c0da5511696256ed40ef04d  ./ontology/bible-o-star.ttl
f54ff8982fb817a4d3e174af23e33bdb95de2d779920ff99c590e325dfb44785  ./ontology/nehemiah-52-shapes.ttl
f05dd51621ad4364fced49db60a9a0284d5aaf855a9f3b16f863f4af0d256a5d  ./ontology/nehemiah-52.ttl
37de03b9299a7dd6910213b5ab9e05bd9a0237504f477a4ea8b689c1aaa9700b  ./ontology/source-ledger.ttl
2da1e95ff3f1ead557d82e15f13ddf2fdecdabb228b680862680c44b2a725b7e  ./queries/bible-o-star.sparql
425c49a2941927b61e3553949d2c528002d386824cf7136e266d0d8cc70192a0  ./queries/README.md
2b2bd49044fa63030190703a1a83e70882a0a8d6d3c11308bd6e650c50152738  ./README.md
4f56c82ea7668f027a0ded5fab63e398a72b2f3f53af146f12596721779b0457  ./receipts/ADVERSARIAL_REVIEW_002.md
a86203a5342de863e4417d358306f34dcc63c52a60bd7734f736ad7d5b445936  ./receipts/BIBLE_O_STAR_001_INSERTION_RECEIPT.md
ee6bbfc3650aa97a81b9253db39620f36469eeee6dd9042d1aa0dd454a4d7f73  ./receipts/BIBLE_O_STAR_001_VALIDATION_RECEIPT.md
785e989cf7b87ccf1615ca19698ed7fec1d7a501ae5b705284c3a349cce210bb  ./receipts/BIBLE_O_STAR_002_INSERTION_RECEIPT.md
4f261a3750d0d354d3853be05426fd32edad09360dbd514bface8eab632d8af9  ./receipts/BIBLE_O_STAR_002_IP_AUDIT.md
c5d6e7efa32605671ea2e874287d3d92e777ddeb9bd935006ae4f5818d1c5375  ./receipts/BIBLE_O_STAR_002_VALIDATION_RECEIPT.md
92f42dbda604fec70ba24c5d83b8f4207a41c30b67aa48f9cbeb8fe85bebfd85  ./receipts/CELL8_CONFORMANCE_RECEIPT.md
1725a4790fb2745e956698b2c8fc274d7dd0628289cfc9028f5afbec416a53d2  ./scripts/validate_bible_o_star.sh
```

## Notes

- Hashes captured at BIBLE_O_STAR_002 checkpoint (2026-06-02).
- Files in `governance/` and `versions/` directories were created as part of
  Cell8 gate closure (A11/A12) and are not included in this pre-002 baseline.
- File count: 52 (pre-A11/A12 files only).
