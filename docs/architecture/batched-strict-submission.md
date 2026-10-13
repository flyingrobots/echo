<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Batched Strict Submission

- **Status:** Implemented.
- **Refines:** [WAL](../topics/WAL.md)
- **Related:** [Fixed timestep invariant](../invariants/FIXED-TIMESTEP.md)

This page states what the runtime does today. A proposal for a deferred
durability mode is not part of it. That proposal and its open decisions live in
[issue #787](https://github.com/flyingrobots/echo/issues/787).

## Problem

A host that runs Echo at a fixed frame rate has a fixed time for each frame.
At 60 frames per second that time is 16.67 ms.

A strict WAL acknowledgement waits for a storage sync. A submission made on its
own is one transaction with one sync. A frame that submits many intents one at
a time therefore waits for many syncs, one after another. The count of syncs,
not the work of the tick, decides whether the frame fits.

The rule of the WAL topic stays in force: Echo may only claim what its WAL can
recover.

## What a host can do

A host submits several envelopes in one call,
`submit_intents_with_runtime_wal_ack`. Echo appends one ordinary intake
transaction for each newly accepted envelope, in order, and then asks the
store to make them durable once. Echo acknowledges the batch only after that
step succeeds. Until then no acceptance of the batch counts as durable and
none can be scheduled.

An envelope that repeats a submission Echo already holds is a duplicate. It
appends nothing and its handle names the existing submission. An empty batch,
and a batch of duplicates only, therefore writes nothing and forces no sync.

`submit_intent_with_runtime_wal_ack` still submits one envelope. A newly
accepted envelope writes one transaction and, on the filesystem store, waits
for one sync. A duplicate returns the existing handle, writes nothing and
syncs nothing.

The bytes in the log are the same transactions that single submission writes.
Recovery reads a batch exactly as it reads single submissions.

## What an acknowledgement means depends on the store

The batch call is the same for every store. Its durability is not.

| Store      | Reported mode      | What the batch call waits for                | What survives a crash                                |
| ---------- | ------------------ | -------------------------------------------- | ---------------------------------------------------- |
| Filesystem | `StrictFilesystem` | Commitment in memory, then one storage sync  | Every acknowledged record, as far as that sync holds |
| In-memory  | `Buffered`         | Commitment in memory; no storage sync exists | Nothing                                              |
| No WAL     | not applicable     | The call returns `RuntimeWalUnavailable`     | Nothing                                              |

An acknowledgement from the filesystem store is a durability claim. It is as
strong as the sync guarantee of the operating system and the device under the
store: Echo asks for the sync and waits for it, and cannot promise more than
that sync delivers. An
acknowledgement from the in-memory store is not: it says the runtime accepted
and ordered the records, and nothing more. `Buffered` is not release
durability. A host must read the reported mode before it tells anyone that an
acknowledged record will survive a crash.

No acknowledgement carries its own durability posture or a log position today.
The mode is a property of the configured store, not of the returned value.

## Crash, refusal and storage failure

[WAL](../topics/WAL.md) owns the complete contract for a batch: what a crash
can leave, what happens when an intent is refused in the middle, what a
storage failure takes back, and what a host must do when it opens an existing
log. This page does not restate it. In short:

- A batch is not atomic across a crash. A prefix can survive, none of it
  acknowledged.
- A refusal before anything is written keeps the earlier intents.
- A storage failure takes back everything the call accepted and stops WAL
  work until the host reopens the WAL.

## Limits of the current implementation

- **A batch has no size limit.** Nothing checks envelope count or encoded
  bytes before the first append.
- **Single submission on the filesystem store is weaker after a failure.**
  After a failed append or sync of a single submission, a host on the
  filesystem store re-reads the log and treats a readable acceptance record
  as accepted. The batch path does not. A host on the in-memory store has no
  log to re-read: it takes the submission back.
- **A tick is a separate sync.** A scheduler pass that produces receipt
  correlations writes one tick transaction and syncs it before outcomes
  become visible. A pass that produces none writes nothing. A strict frame
  that submits a batch and then ticks therefore waits for two syncs.
- **All WAL writes happen on the caller's thread.** No writer thread exists.

Work on these limits is tracked in
[issue #787](https://github.com/flyingrobots/echo/issues/787).

## Alternative not chosen

**One transaction that carries many acceptances.** It would make a batch
atomic across a crash. It needs a new transaction shape and new recovery
rules. Batching ordinary transactions gives the same sync count without a
format change.

## Evidence

- API: `submit_intents_with_runtime_wal_ack` in
  [`crates/warp-core/src/trusted_runtime_host.rs`](../../crates/warp-core/src/trusted_runtime_host.rs).
- Witnesses, in
  [`crates/warp-core/tests/trusted_runtime_host_loop_tests.rs`](../../crates/warp-core/tests/trusted_runtime_host_loop_tests.rs):
    - `batched_submission_makes_every_acceptance_durable_with_one_sync`
    - `batched_submission_handles_duplicates_and_an_empty_batch`
    - `batched_submission_refused_before_storage_keeps_earlier_intents_and_the_wal`
    - `batched_submission_refused_before_storage_keeps_an_in_memory_wal_usable`
    - `batched_submission_append_failure_fails_closed_until_the_wal_is_reopened`
    - `batched_submission_sync_failure_rolls_back_intake_and_fails_closed`
