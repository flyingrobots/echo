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
`submit_intents_with_runtime_wal_ack`. Echo appends the ordinary intake
transaction of each envelope, in order, and then asks the store to make them
durable once. Echo acknowledges the batch only after that step succeeds. Until
then no acceptance of the batch counts as durable and none can be scheduled.

`submit_intent_with_runtime_wal_ack` still submits one envelope with one
transaction and one sync.

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

## A batch is not atomic across a crash

Each transaction is atomic through its own commit record, and a torn tail is
truncated as it is for single submissions. A crash before the sync can
therefore leave a prefix of the batch. Echo acknowledged none of it, so each
surviving submission recovers as accepted and pending. That is the same
posture as a single submission whose process died after its commit and before
its acknowledgement.

## Failure inside a batch

An envelope can fail in the middle of a batch. The result depends on where:

- **Before anything is written for it.** The runtime or the WAL refuses the
  envelope. The envelopes before it are synced and stay accepted. The call
  returns the refusal and no acknowledgement. A caller that submits an
  earlier envelope again receives its existing submission.
- **While it is written, or at the sync.** Storage failed. Echo takes back
  every submission the call accepted, acknowledges nothing, and refuses
  further WAL work with `StorageFailed` until the host reopens the WAL.

When the final sync fails, the records may still be readable in the log,
because the operating system can hold them. A readable record is not a durable
one, and a later sync cannot vouch for records that an earlier sync failed to
write. The running host therefore does not retry and does not reconcile itself
from the log. After the failure it does not answer recovery reads or offer its
cached causal frontier or evidence catalog.

A host that opens an existing log, and a writer that takes one over, sync its
segment files before reading them as recovered history. That sync does not
turn a readable record into a strict acknowledgement.

## Limits of the current implementation

- **A batch has no size limit.** Nothing checks envelope count or encoded
  bytes before the first append.
- **Single submission is weaker after a failure.** After a failed append or
  sync of a single submission, the host still re-reads the log and treats a
  readable acceptance record as accepted. The batch path does not.
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
