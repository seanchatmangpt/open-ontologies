# Muster Ledger — Bible O*

## Doctrine
Counting the people is a Muster Ledger, not vanity metrics.
Nehemiah 7 counts the returned exiles by family and service.
The MusterLedger records who is doing the work, not how impressive the numbers look.

## bos:MusterLedger
The MusterLedger is a census of active builders:
- Who is assigned to which WallSection
- Which Builder is accountable
- What their gate assignment is
- What receipts they have emitted

## bos:MusterLedgerRecord fields
- bos:hasBuilder — named accountable worker
- bos:assignedToGate — their gate assignment
- bos:buildsWallSection — their wall section responsibility
- bos:hasMusterReceipt — proof of assignment
- bos:hasMusterRecord — association with the registry
- bos:hasSource — canonical scripture reference

## PeopleGate Refusal
There is no "PeopleGate." People are not a gate.
The Muster Ledger tracks people. Gates admit or refuse motion.

## MessengerGate Refusal
There is no "MessengerGate." Messengers are Couriers. Couriers operate the Courier Layer.
The Courier Layer is not a gate; it is a channel.

---

## Extended Doctrine

### Muster vs. Vanity Metrics

A vanity metric is a number that flatters without obligating. "We have 10,000 users" is
a vanity metric if none of them are named, none have assigned sections, and none are
accountable for completion.

A muster record obligates:
- The builder is named
- The section is assigned
- The work is traceable to a canonical source
- Completion or incompletion is visible in the receipt chain

Nehemiah 3 is a muster. Every verse names the builder and the section. The count appears
incidentally because the named entries produce it.

### Absence Is Visible

Neh.3.5 records that the Tekoite nobles "did not put their necks to the work of their Lord."
Refusal is recorded in the muster with the same precision as completion. A muster that
cannot record absence is not a muster — it is a press release.

### Muster and Genealogy (Nehemiah 7)

Nehemiah 7 uses the muster mechanism for identity verification: those who could not prove
their genealogy were excluded from the priesthood (Neh.7.64-65). Two muster uses:

- **Neh.3 muster**: assignment accountability — who builds what
- **Neh.7 muster**: identity accountability — who is who they claim to be

Both uses share the same structural requirement: the name must be traceable to a source.
An unnamed entry fails both muster types.

### Ontological Role Summary

| Property | Value |
|---|---|
| Class | `bos:MusterLedgerRecord`, `bos:MusterRegistry` |
| Ledger purpose | Named accountability registry |
| Is it a headcount? | No — count is a side effect of named entries |
| Builder required? | Yes — unnamed entries are not valid muster records |
| Absence recorded? | Yes — "did not put their necks to the work" is a valid entry |
| Genealogy muster | Variant use: identity verification, not just section assignment |
