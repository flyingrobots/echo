<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Retro — 0004 strand-contract

## What shipped

The strand contract: types, registry, invariant document, and tests.

**Code:** `crates/warp-core/src/strand.rs`

- `StrandId` — domain-separated hash (prefix `b"strand:"`)
- `BaseRef` — immutable fork coordinate with exact semantics
  (fork_tick = last included tick, commit_hash at fork_tick,
  boundary_hash = output boundary, provenance_ref handle)
- `SupportPin` — braid geometry placeholder (empty in v1)
- `DropReceipt` — hard-delete proof
- `Strand` — relation descriptor with no lifecycle field
- `StrandRegistry` — `BTreeMap<StrandId, Strand>` with CRUD
- `StrandError` — typed error enum

**Invariant document:** `docs/invariants/STRAND-CONTRACT.md`

Ten invariants (INV-S1 through INV-S10) covering immutable base,
own heads, session scope, manual tick, complete base_ref, inherited
quantum, distinct worldlines, head ownership, empty support_pins,
and clean drop.

**Tests:** 14 Rust integration tests + 12 shell assertions = 26 total,
all passing.

## Playback witness

### Human playback

| #   | Question                                        | Answer | Witness                              |
| --- | ----------------------------------------------- | ------ | ------------------------------------ |
| 1   | Does create_strand return correct fields?       | Yes    | 14 Rust tests pass                   |
| 2   | Is base_ref pinned exactly?                     | Yes    | inv_s5 test                          |
| 3   | Are strand heads Dormant/Paused?                | Yes    | inv_s4 test                          |
| 4   | Are strand heads excluded from runnable set?    | Yes    | inv_s4_s10 integration test          |
| 5   | Does drop remove everything and return receipt? | Yes    | registry_remove + drop_receipt tests |

### Agent playback

| #   | Question                                     | Answer | Witness                               |
| --- | -------------------------------------------- | ------ | ------------------------------------- |
| 1   | Does Strand struct have all contract fields? | Yes    | Type definition + v1_cardinality test |
| 2   | Is TTD mapping documented?                   | Yes    | Invariant doc cross-references        |
| 3   | Does list_strands filter correctly?          | Yes    | registry_list_by_base test            |

Full test output in `witness/`.

## Drift check

- **Design drift:** The first design draft had `StrandLifecycle`
  (Created → Active → Dropped). Human review caught this as a second
  scheduler truth. Eliminated: strand exists in registry = live,
  not in registry = gone. Heads are the single source of truth.
- **Drop drift:** First draft both "transitioned to Dropped" and
  "removed from registry." Human review caught the inconsistency.
  Fixed to hard-delete with `DropReceipt`.
- **BaseRef drift:** First draft had the right fields but not the
  right precision. Human review required exact coordinate semantics
  (fork_tick = last included tick, boundary_hash = output boundary).
  Fixed with `provenance_ref` handle added.
- **Invariant drift:** Original six invariants expanded to ten after
  review: added INV-S7 (distinct worldlines), INV-S8 (head
  ownership), INV-S9 (empty support_pins), INV-S10 (clean drop).

## New debt

- `create_strand` and `drop_strand` as orchestrated operations
  (provenance fork → head creation → registry insert, with rollback)
  are defined in the design doc but not yet implemented as a single
  API. The types and registry exist; the wiring through the
  coordinator or a standalone service is future work.
- `LocalProvenanceStore` has no `remove_worldline` method. Drop
  currently relies on session-scoped lifetime. If mid-session drop
  cleanup is needed, this must be added.

## Cool ideas

- Strand creation could emit a `ProvenanceEventKind::CrossWorldlineMessage`
  as a "strand created" announcement, visible in the debugger's
  timeline. This would give TTD a provenance-native anchor for
  strand creation without inventing a new event kind.
- A `strand diff` command (like `git diff` between branches) would
  let the debugger show exactly what changed because of a strand's
  speculative ticks. This is the `compareStrand` operation from
  git-warp, applied to Echo's richer provenance model.
