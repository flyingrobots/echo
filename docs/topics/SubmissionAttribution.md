<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Submission Attribution

**Owns:** which accepted submissions a tick decided, and how Echo proves that
its own records of those decisions are complete.
**Used by:** [causal receipt history](CausalReceiptHistory.md), intent outcome
lookup, WAL recovery and history restore.
**Carried by:** the [WAL truth boundary](WAL.md) (`EPRV0003` retained ticks).
**Hash contract:** [commit hash v3](../spec/merkle-commit.md) binds the section.
**Decision record:** [#873](https://github.com/flyingrobots/echo/issues/873).

Every tick Echo commits now says, in its own retained receipt, which accepted
submissions it decided and how. Before this, that fact lived only in a runtime
index of receipt correlations. Plain ticks left a graph node per request, and
Action ticks left nothing an index could be checked against. A lost
correlation therefore made history silently shorter. Attribution moves the fact
into committed history and turns the correlation index into a cache that Echo
checks and can rebuild.

Most of what this page describes runs inside Echo with no application
involvement. Applications see it mainly as more precise outcomes and history
pages they can trust.

## The section

A tick receipt (`TickReceipt`) lists the rewrite candidates a tick tried and
whether each applied. From receipt format version 3 it also carries an
attribution section:

```rust
pub struct TickSubmissionAttribution {
    pub submission_id: Hash,
    pub decision: TickSubmissionDecision,
}

pub enum TickSubmissionDecision {
    /// Indices into the receipt's entries. Never empty; strictly increasing.
    ReceiptEntries(Vec<u32>),
    /// The tick committed the submission, but no rule matched it.
    NoMatchingRule,
}
```

Rules enforced by `TickReceipt::with_attribution` (`receipt.rs`):

- records are in strictly increasing submission order, so each submission
  appears once;
- a decision refers to receipt entries rather than copying their dispositions;
- an empty entry list is refused, so "no entries" never stands in for "no
  match"; that is its own variant;
- no two submissions may claim one receipt entry.

An empty section is valid and is not the same as no section. It means the tick
decided no submission, as a direct host commit does. No section means a legacy
tick written before attribution existed.

## Identity

The section names the **submission ID**, the identity Echo already uses for
outcome lookup:

- `submission_id = H("echo.intent-submission.v0", worldline, head, ingress_id)`
  (`coordinator.rs`, `derive_intent_submission_id`);
- `ingress_id` hashes intent kind, bytes and causal parents, not the target, so
  one request sent to two heads is two submissions;
- each tick commits for exactly one writer head, so within a tick the submission
  ID names exactly one accepted submission.

**Retries and repeats.** Intake is idempotent per head
([canonical inbox sequencing](../spec/canonical-inbox-sequencing.md)). Retrying
the same content on the same head returns the existing submission as a
`Duplicate`, commits no tick and adds no history item. A deliberate repeat needs
different content: different intent bytes, or a causal parent citing earlier
history. Either one changes the ingress ID and so creates a new submission.

## Sealing: how a tick builds its section

The section is built from each tick's own decision process, never from a
correlation index. A tick whose decisions and receipt disagree is not sealed.

| Commit path                                                 | How the section is built                                                                                                                                                                                                                | Refuses to seal when                                                                                        |
| ----------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Plain runtime tick (`Engine::commit_attributed_with_state`) | Records whether each submission's command rule matched, then takes the entries scoped to its request node `NodeId(ingress_id)`. Entries are grouped by scope once: `O(r log s + n log s)` for `r` entries, `s` scopes, `n` submissions. | A matched submission has no entry; an unmatched one has entries; an entry belongs to no admitted submission |
| Action batch (`commit_scheduler_action_batch_to_state_v1`)  | Each candidate is attributed to the receipt entry at its own position.                                                                                                                                                                  | Candidates and entries differ in number                                                                     |
| Direct host commit (`commit_prepared_to_state`)             | Decides no submission, so it records an empty section.                                                                                                                                                                                  | Never; the section is fixed                                                                                 |

The scheduler derives each submission ID from the head and ingress ID by the
identity law. It never looks one up, so a damaged index cannot change what is
committed. `Engine::commit_with_state` stays unattributed for engine-direct
callers, which have no submissions behind them.

## Format, binding and legacy

| Receipt                                         | Receipt digest | Retained record | Commit hash |
| ----------------------------------------------- | -------------- | --------------- | ----------- |
| Commit-bound section (every new scheduler tick) | version 4      | `EPRV0003`      | v3          |
| Section retained before commit binding existed  | version 3      | `EPRV0002`      | v2          |
| No section (legacy tick)                        | version 2      | `EPRV0001`      | v2          |

- A retained record cannot change version by relabelling its magic.
- A legacy history can be extended by attributed ticks. A legacy tick's commit
  hash is the same with or without a section.
- **Binding.** Attribution is bound transitively into commit identity:
  commit hash → receipt digest → attribution, as the owner ruled on #873. A
  commit-bound receipt (format 4) is committed with commit hash v3, which
  hashes the receipt digest alongside v2's inputs (`compute_commit_hash_v3`).
  An explicit format marker, never the mere presence of a section, selects the
  version (`compute_commit_hash_for_receipt`), so `EPRV0002` and `EPRV0001`
  ticks keep their v2 identities. Replacing the attribution changes the
  receipt digest and so the commit hash; stripping the binding selects v2,
  which cannot reproduce the recorded v3 hash. Retained-commit validation and
  replay recompute the selected version. The canonical contract is
  [merkle-commit](../spec/merkle-commit.md), Decision 3a.

## How Echo uses it on its own

These paths run on every relevant call. No application has to ask for them,
opt in, or supply anything.

### 1. Outcome lookup

`observe_intent_outcome` and `observe_app_intent_outcome` answer "what happened
to my submission?". For an attributed tick, the answer comes from the receipt
alone: the submission's record names its entries, or `NoMatchingRule`.

- **Before:** an Action's outcome was matched to its entry by position, which
  needed every correlation of the tick. One lost sibling correlation broke the
  lookup for every Action in that tick.
- **Now:** one submission's outcome depends only on its own record. The
  correlation must also obey the identity law, so a corrupted correlation cannot
  borrow another receipt's decision.
- `IntentOutcome::NoMatchingRule` reports "no rule matched" as a decision with
  its receipt coordinate, not as missing evidence. For a provider-bound
  submission it stays a provider runtime fault, because an installed provider
  refusing its own operation is never an ordinary non-match.
- Legacy ticks keep the scope and positional rules.

### 2. Receipt history

Every history page walks retained ticks and treats each tick's section as the
authority. Echo checks the correlation index against it, tick by tick:

- an attributed, ticketed submission with no correlation is
  `AttributedSubmissionUncorrelated`. This catches a lost correlation even when
  every correlation of the tick is gone, which an index scan could never see.
  The page walks each tick's attribution and index entries together, in
  submission order, so every index entry is checked and a page pays only for
  the records it reaches;
- a correlation the tick does not attribute is `UnattributedCorrelation`;
- an attributed submission that was never ticketed is
  `SubmissionWithoutReceiptIdentity`;
- a page's claim names exactly the ticks it inspected: `Established` when every
  one is attributed and commit-bound, `EstablishedWithoutCommitBinding` when
  every one is attributed but some predate commit binding, and
  `UnestablishedForLegacyTicks` otherwise.

See [causal receipt history](CausalReceiptHistory.md) for paging, budgets and
the cursor.

### 3. Recovery and history restore

When `restore_causal_runtime_history` rebuilds correlations from durable
records, each one must be named by its tick's attribution. That includes a
`NoMatchingRule` submission in a tick that also applied others, which the
legacy scope check would wrongly refuse. The retained-provenance codec
re-encodes every decoded tick and refuses any record that is not canonical.

### 4. A rebuildable index

Because attribution is committed, the five correlation maps are a cache:

- every attributed tick says which correlations must exist;
- recovery and every history page check the cache against it;
- a missing or wrong derived entry can be recognised. Missing authoritative
  submission or envelope material, or conflicting durable evidence, is reported
  as a typed obstruction or recovery failure, never repaired silently.

## What applications can do with it

The remaining uses are explicit application reads:

- **"What happened to my request?"** Poll the outcome. A request no rule
  matched is now a definite answer.
- **Retry with confidence.** A retry after a timeout returns the original
  submission and never creates a second history item, so an application can
  retry blindly.
- **Audit and reconciliation.** Page through receipt history at a fixed basis.
  An `Established` page proves the history it covers is complete and bound into
  commit identity; `EstablishedWithoutCommitBinding` proves completeness with
  weaker binding. That is what an audit trail or a post-crash reconciliation
  needs.
- **Undo and redo.** Read history, choose a transition by contract policy,
  derive and submit its inverse
  ([contract inverse admission](ContractInverseAdmission.md)). Completeness
  means the transition chosen is not one Echo silently forgot.
- **Diagnosis.** Typed obstructions say which link failed: the tick, the
  correlation, the submission record, or the transition.
- **Exchange between participants.** Attributed ticks carry their own account
  of which submissions they decided, so a receiving participant can check a
  history it did not produce. Transport is planned through Continuum, not
  inside Echo.

## Costs

Measured costs are not yet recorded; these are derived from the format.

- **Storage:** per decided submission, a 32-byte submission ID and a 1-byte
  tag, plus an 8-byte count and 4 bytes per entry when the decision names
  entries; one 8-byte record count per tick. A typical Action or matched
  request with one entry adds 45 bytes to its retained tick.
- **Memory:** retained ticks are held in memory with the same overhead. The
  correlation maps are unchanged.
- **Commit CPU:** plain-tick sealing builds an ordered map of entries by
  scope, `O(r log s)`, then does one lookup per submission, `O(n log s)`. Action sealing is one entry per
  candidate. Both hash the section once into the receipt digest.
- **Query CPU:** outcome lookup is a binary search in one tick's section. A
  history page is bounded by its tick and record budgets.

## Limits

- **Legacy ticks** cannot prove completeness. History marks pages that
  inspected them `UnestablishedForLegacyTicks`.
- **No-match across restart is not yet proven.** A ticketed no-match outcome is
  still unrecordable in the WAL tick record (`TickOutcomeUnavailable`). The
  owner ruled that it must survive restart, with outcome lookup and history
  agreeing before and after recovery; #873 depends on that work.
- **Unticketed submissions** decided by a tick have no receipt identity, so
  history reports them as `SubmissionWithoutReceiptIdentity` instead of
  returning them.
- **Causal-parent count** is bounded only on the read side for now (#874).

## Evidence

| Claim                                                                                        | Test                                                                                                                                      |
| -------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Legacy receipt digest unchanged; the section changes it; malformed shapes refused            | `receipt::tests`                                                                                                                          |
| Plain seal and its refusals                                                                  | `engine_impl::tests::attribution_seal`                                                                                                    |
| Plain ticks attribute entries and no-match                                                   | `coordinator::tests::a_plain_tick_attributes_each_submission_to_its_receipt_entries`, `a_plain_tick_records_a_submission_no_rule_matched` |
| Retry adds nothing; repeat is new                                                            | `coordinator::tests::an_idempotent_retry_adds_no_history_item_and_an_intentional_repeat_does`                                             |
| `EPRV0001`, `EPRV0002` and `EPRV0003` formats; mixed history                                 | `coordinator::tests::retained_ticks_keep_their_format_and_a_legacy_history_extends_with_attributed_ticks`                                 |
| Outcomes resolve from the receipt alone                                                      | `coordinator::tests::attributed_outcomes_resolve_from_the_receipt_alone`, `an_attributed_no_match_is_a_decision_and_ambiguity_is_refused` |
| Restore accepts an attributed no-match                                                       | `coordinator::tests::receipt_correlation_restore_accepts_an_attributed_no_match_beside_an_applied_submission`                             |
| Lost correlations, unattributed correlations, budgets, cursors                               | `causal_receipt_history::tests`                                                                                                           |
| Action attribution, paging inside one tick, and recovery                                     | `executable_operation_pipeline_tests::an_action_tick_attributes_each_submission_and_the_attribution_recovers`                             |
| Attribution is bound into commit identity; substitution under the original commit is refused | `coordinator::tests::substituting_attribution_under_a_ticks_commit_fails_validation`                                                      |
| Ticks attributed before commit binding keep their v2 identity                                | `coordinator::tests::an_attributed_tick_retained_before_commit_binding_still_validates`                                                   |
