<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# WAL Durability Modes and Batched Submission

- **Status:** Proposed design. Batched submission is implemented; deferred
  mode and acknowledgement postures are **not implemented**. The section
  [Current implementation](#current-implementation) states what the runtime
  does today.
- **Proposal date:** 2026-10-12
- **Refines:** [WAL](../topics/WAL.md)
- **Related:** [Sliced readings and copy-on-write strands](sliced-readings-and-copy-on-write-strands.md),
  [Fixed timestep invariant](../invariants/FIXED-TIMESTEP.md)

## Problem

A host that runs Echo at a fixed frame rate has a fixed time for each frame.
At 60 frames per second that time is 16.67 ms.

A strict WAL acknowledgement waits for a storage sync. A submission made on its
own is one transaction with one sync, and each scheduler pass that produces
WAL-backed outcomes is another. A frame that submits many intents one at a time
therefore waits for many syncs, one after another. The count of syncs, not the
work of the tick, decides whether the frame fits.

The rule of the WAL topic stays in force: Echo may only claim what its WAL can
recover. This page changes when Echo waits and what an acknowledgement says.
It does not change what Echo may claim.

## Decision

A host chooses one of three durability modes for a runtime.

| Mode     | An acknowledgement means                         | The caller waits for                |
| -------- | ------------------------------------------------ | ----------------------------------- |
| None     | Accepted in memory. Nothing is recoverable.      | Nothing                             |
| Deferred | Accepted, ordered, and handed to the WAL writer. | Room in the writer's queue, if full |
| Strict   | Durable. Recovery will rebuild it.               | One storage sync                    |

A deferred caller normally waits for nothing. When the writer's queue is full
it waits for room, and that wait ends in one of two ways: room appears, or
the host declares the writer failed and the caller receives that failure. A
deferred acknowledgement is therefore not a promise of a bounded wait until
the limit in [Open decisions](#open-decisions) is set.

Strict mode has batched submission. Deferred mode is new. None is the
existing behavior of a host that does not enable a WAL.

### Batched submission

A host submits several envelopes in one call. Echo appends the ordinary intake
transaction of each envelope, in order, and then syncs storage once. Echo
acknowledges the batch only after that sync.

The bytes on disk are the same transactions that single submission writes
today. Recovery does not change.

A batch is not atomic across a crash. Each transaction is atomic through its
own commit record, and a torn tail is truncated as it is today. A crash before
the sync can therefore leave a prefix of the batch. Echo acknowledged none of
it, so each surviving submission recovers as accepted and pending. That is the
same posture as a single submission whose process died after its commit and
before its acknowledgement.

A batch has a size the host sets: a limit on envelopes and on encoded bytes.
Echo checks the whole batch against that limit before it appends anything. A
batch over the limit is refused as a whole with a typed over-budget result.

An envelope can fail in the middle of a batch. The result depends on where:

- **Before anything is written for it.** The runtime or the WAL refuses the
  envelope. The envelopes before it are synced and stay accepted. The call
  returns the refusal and no acknowledgement. A caller that submits an
  earlier envelope again receives its existing submission.
- **While it is written, or at the sync.** Storage failed. Echo takes back
  every submission the call accepted, acknowledges nothing, and refuses
  further WAL work until the WAL is reopened.

When the final sync fails, Echo acknowledges nothing. The records may be
readable in the log, because the operating system can still hold them, but a
readable record is not a durable one. A later sync cannot vouch for records
that an earlier sync failed to write, so the running host does not retry: it
takes the batch back, refuses further WAL work, and must reopen the WAL. A
host that opens a log syncs it before reading it as recovered history. It does
not turn a readable record into a strict acknowledgement.

### Deferred mode

In deferred mode a writer thread inside the Echo process owns the WAL store
and its writer-epoch lease. The runtime hands it records through a bounded
queue and does not touch storage itself.

An acknowledgement names the record by identity: its writer epoch and its
transaction. It also carries the record's log position. It states that the
record is accepted and not yet durable.

Identity matters because a position can be reused. If a crash loses an
accepted record, recovery resumes from the last recovered position and a
different record can take the lost one's place. A wait for durability is
satisfied only by the record with the same identity, and recovery reports a
lost acknowledgement as lost.

The writer appends queued records in order and syncs them in groups. After
each sync it publishes the durable position: every record at or before that
position will be rebuilt by recovery. A host may read the durable position, or
wait until it passes a given acknowledgement.

A crash in deferred mode may lose records after the durable position. It
does not always lose them: the operating system may have written a further
prefix that Echo never promised. Recovery rebuilds a valid prefix of what the
runtime accepted, at least up to the durable position and possibly beyond it.
It never rebuilds a gap. A caller must therefore treat a record after the
durable position as indeterminate until recovery reports it.

## Laws

1. **An acknowledgement states its posture.** Strict and deferred
   acknowledgements are different values. A caller cannot mistake accepted for
   durable.
2. **Durability reports are observations.** The durable position, queue depth
   and writer timing never influence which intents are admitted, how a tick
   is scheduled, or what a tick computes. Two runs that are offered the same
   intents in the same order produce the same history in every mode.
3. **Order is fixed at acceptance.** The log order is the order in which the
   runtime accepted records. The writer never reorders, merges or drops.
4. **Nothing is dropped silently, and pressure never refuses.** When the
   queue is full, the caller waits. A full queue never refuses an intent,
   because that would let writer timing decide what is admitted and would
   break law 2. When the writer fails, that is an internal fault: Echo did
   not uphold its own durability. The runtime records the fault and stops
   accepting work that would need the WAL. A caller whose work is refused for
   that reason receives an obstruction that names the fault. The two stay
   separate typed values, as [Obstructions](../topics/Obstructions.md)
   requires: the fault needs trusted recovery, and the obstruction tells the
   caller that support is unavailable. The runtime does not continue in a
   weaker mode.
5. **The log is an ordinary Echo WAL.** Every mode that writes a WAL writes
   the same records. One recovery path reads all of them.
6. **One writer.** In every mode that writes a WAL, exactly one writer holds
   the epoch lease. Deferred mode moves that writer to a thread. It does not
   add a second one. Mode None has no WAL and no lease.
7. **A mode is declared.** The mode is part of host configuration and is
   visible in evidence. A receipt produced without a WAL, or before its
   record was durable, does not claim strict durability.

## Alternatives not chosen

- **An external WAL process on a port.** It gives the same speed as deferred
  mode and adds a wire protocol, a second process to deploy and a second
  holder of storage authority. Its failures differ from an in-process
  writer's. If the Echo host dies after handing a record over, the external
  writer can still sync it, where an in-process writer dies with the host. If
  the external writer dies, records it had accepted and not written are lost
  while the host runs on. Each case needs its own acknowledgement and
  recovery rule, and an acknowledgement would have to say which process it
  binds. That is a larger contract than this design needs.
- **One transaction that carries many acceptances.** It makes a batch atomic
  across a crash. It needs a new transaction shape and new recovery rules.
  Batched submission with ordinary transactions gives the same sync count
  without a format change. This remains an open decision if a host needs
  all-or-nothing batches.

## Open decisions

- Whether a strict frame should cover its submissions and its tick with one
  sync instead of two.
- How long a caller may wait on a full deferred queue before the host treats
  the writer as failed.
- Whether a host may change mode while a runtime is live, and what evidence
  the change leaves.

## Current implementation

The runtime implements batched submission. It does not implement deferred
mode or acknowledgement postures.

- A host without a WAL runs and recovers nothing. This is mode None.
- `submit_intent_with_runtime_wal_ack` writes one submission-intake
  transaction and returns after that transaction's commit record is synced.
- `submit_intents_with_runtime_wal_ack` accepts several envelopes, appends one
  ordinary intake transaction for each, and returns after one sync covers all
  of them. Until that sync succeeds no acceptance of the batch counts as
  durable or can be scheduled. [WAL](../topics/WAL.md) states its failure
  rules.
- A batch has no size limit. Nothing checks envelope count or bytes before
  the first append.
- After a storage failure in a batch the adapter refuses further WAL work with
  `StorageFailed` until the host reopens the WAL. It does not reconcile itself
  from the log, answer recovery reads, or offer its cached causal frontier or
  evidence catalog.
- A host that opens an existing log, and a writer that takes it over, sync its
  segment files before reading them as recovered history.
- After a failed append or sync of a _single_ submission, the host still
  re-reads the log and treats a readable acceptance record as accepted. That is
  weaker than the batch rule and than the rule this page states.
- A scheduler pass that produces receipt correlations writes one tick
  transaction and syncs it before outcomes become visible. A pass that
  produces none writes nothing. The filesystem store refuses a pass that would
  need more than one tick transaction.
- A strict filesystem commit forces one sync: the commit record.
- `WalDurabilityMode` names `StrictFilesystem`, `StrictObjectStore`,
  `Buffered`, `ReadOnlyRecovery` and `Disabled`. The trusted host reports
  `Buffered` for its in-memory store and `StrictFilesystem` for its filesystem
  store. No acknowledgement carries a durability posture or a log position.
- All WAL writes happen on the caller's thread. No writer thread exists.

Implementation state is owned by
[issue #787](https://github.com/flyingrobots/echo/issues/787), not by this
page.
