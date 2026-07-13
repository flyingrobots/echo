<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# RE-034: Separate Tick Receipt Content From Causal Identity

Status: core identity and durable transport resolved; inverse-admission witness
pending.

## Problem

`TickReceipt::digest()` intentionally commits to canonical candidate outcomes
without transaction identity. Two lawful transitions can therefore have the
same receipt digest. Current causal-parent and recovered reverse-lookup
surfaces use that content digest as though it uniquely identified an admitted
receipt event.

## Why It Matters

Undo, redo, compensation, and other contract-defined inverse intents must cite
one admitted transition, not an equivalence class of receipts with the same
content. Following a content digest alone can make provenance ambiguous after
repeated structurally identical operations.

## Desired Shape

- Define a typed causal receipt reference that binds the receipt content digest
  to its admitted worldline, worldline tick, global tick, commit hash, and
  ticketed-ingress or submission identity.
- Keep `TickReceipt::digest()` as the transaction-independent content
  commitment used by decision verification.
- Make causal-parent citations and reverse child lookup use causal receipt
  references rather than bare content digests.
- Migrate WAL, WSC, outcome, and contract-intent adapters without inventing
  application-level undo semantics in Echo.

## Acceptance Tests

- [x] `identical_receipt_content_has_distinct_causal_receipt_refs`
- [ ] `inverse_intent_resolves_one_admitted_transition_after_restart`
- [x] `causal_parent_lookup_does_not_alias_identical_receipt_content`
- [x] `legacy_bare_receipt_digest_is_reported_as_ambiguous`

The remaining witness belongs to contract inverse admission. It must resolve a
specific `CausalTickReceiptRef` after restart and must not reintroduce a
content-digest lookup or application-owned process cache.
