<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0025: Scheduler-Owned Executable-Operation Actions

- **Status:** Accepted
- **Date:** 2026-07-24
- **Amends:** ADR 0023

## Context

ADR 0023 introduced Echo-interpreted executable-operation packages so
application semantics no longer need a native callback. Its first runtime
crossing nevertheless exposes a second execution lifecycle:

```text
admit invocation
prepare privately
commit prepared operation directly
```

That crossing proved exact package resolution, bounded private evaluation,
typed obstruction, patch validation, and durable recovery. It is not the
permanent application route. Echo's public write lifecycle is an `Action`
accepted into causal ingress, selected by the scheduler, and decided as part of
one atomic `Tick`.

Keeping direct operation commit as a peer lifecycle would let application or
host-adapter code choose when evaluation runs, bypass normal Lane/head
selection, and publish a singleton consequence outside the scheduler's
multi-Action decision. That would preserve the same callback-shaped authority
problem Edict was introduced to remove, merely behind a different artifact.

The current scheduler and WAL also assume different cardinalities:

- one head commit may decide several ingress envelopes;
- `TrustedRuntimeHost::tick_once` writes one scheduler-WAL transaction per
  receipt correlation;
- the filesystem WAL rejects more than one such transaction because the group
  would not be failure-atomic.

One Tick must be one durable transaction, regardless of its Action count.

## Decision

### Executable operations enter through canonical Actions

Echo reserves one stable `IntentKind` for executable-operation Actions. Its
payload is exactly one canonical `EchoOperationInvocationV1`; package meaning
is never copied into the envelope and no native callback is attached.

Application-facing code may only submit that envelope through the ordinary
submission boundary. An acknowledgement means the canonical Action and its
acceptance evidence have committed to the WAL. It does not mean the Action has
executed.

The runtime owner configures invocation-admission policy. After acceptance,
Echo resolves the exact installed package, issues ticketed runtime ingress, and
places the Action in the normal target head inbox. A restart reconstructs the
accepted pending Action from retained submission material; admission and
staging are deterministic and repeatable.

### The scheduler owns evaluation

Only scheduler Tick construction invokes the private bounded evaluator for an
Action. The evaluator remains private and pure with respect to the parent
worldline:

- success returns one complete prepared candidate;
- obstruction returns typed evidence and no parent-visible operations;
- neither result mutates the parent state.

The direct `prepare_echo_operation_v1` and
`commit_prepared_echo_operation_v1` methods remain temporarily available only
as explicitly transitional compatibility/test seams. They are not the
application architecture and will be removed after existing convergence
witnesses migrate.

### Legacy and executable ingress never share an evaluator batch

Provider/native ingress and executable-operation Actions are different
execution categories. A head inbox may contain both while migrations are in
flight, but one Tick candidate batch is homogeneous.

The scheduler examines the lowest canonical ingress identity and drains only
that entry's execution category, subject to the existing inbox budget. Other
categories remain pending for a later Tick. This is deterministic, preserves
global inbox order at category boundaries, and makes it impossible for an
executable Action to fall through to a native callback engine.

### One exact parent basis, per-Action application propositions

Actions selected into one Tick share the exact Echo parent coordinate:

- writer head;
- worldline tick;
- committing global-tick predecessor;
- state root;
- parent commit identity.

Each Action retains its own application-basis proposition. Two independent
Actions over different domain values therefore need not have byte-identical
complete `EchoOperationEvaluationBasisV1` values. This amends ADR 0023's
informal statement that composed preparations share identical complete basis
bytes: they share the runtime parent fields, while the separately typed
application proposition remains candidate-specific.

### Tick construction is atomic

Prepared candidates are considered in canonical Action order. The scheduler
reserves their actual footprints against already accepted candidates:

- independent candidates are applied to a private successor state;
- footprint conflicts produce a rejected Action outcome naming earlier
  applied blockers;
- evaluator obstructions produce a typed obstructed Action outcome with no
  blockers and no mutation.

All accepted candidate operations are canonicalized into one
`WarpTickPatchV1`. Tick construction produces one snapshot, one Tick receipt
with one entry per Action, one provenance entry, and one worldline frontier
advance. Failure while evaluating, composing, applying, or validating discards
the private successor in full.

The generic Tick receipt classifies scheduler disposition. A separate typed
executable-Action outcome binds the exact invocation, preparation or
obstruction, Tick identity, and committed member consequence. Application
semantics are not inferred from a generic receipt label.

### One Tick is one WAL transaction

A scheduler Tick WAL transaction contains, in canonical Action order:

```text
TickReceiptRecorded + ReceiptCorrelationRecorded + ActionOutcomeRecorded
... repeated for each Action ...
RuntimeStateDeltaRecorded
```

Every receipt record cites the same Tick receipt content digest and the
submission-specific causal coordinate. The state delta appears exactly once
because the Tick has exactly one atomic state consequence.

The WAL transaction is committed before the live runtime, frontier, provenance,
Action outcomes, or receipts are published. On append failure, live state is
restored to the accepted-pending posture. Recovery validates the whole group,
reconstructs the state transition once, and restores every per-Action outcome
and receipt correlation.

## Consequences

- Application code cannot invoke executable evaluation or direct commit.
- Two independent executable-operation Actions can contribute to one
  scheduler-owned Tick.
- A typed obstruction is durable evidence and cannot hide a state mutation.
- Filesystem durability no longer depends on several scheduler transactions
  being atomically appended as an external batch.
- Existing executable package, program, invocation, and direct singleton
  receipt bytes remain unchanged.
- Native/provider callback ingress remains compatibility infrastructure and is
  not extended by this decision.

## Acceptance

The implementation is accepted only with executable witnesses that prove:

1. Action acknowledgement follows WAL commit.
2. Accepted pre-Tick Actions survive restart as pending work.
3. Submission and runtime admission do not evaluate or mutate state.
4. Only scheduler Tick construction invokes private evaluation.
5. Two independent Actions produce one Tick and one atomic state consequence.
6. Tick construction or WAL failure publishes no partial state or receipt.
7. Typed obstruction contributes no operations.
8. The decided Tick is durable before frontier and receipt publication.
9. Fresh-host recovery reconstructs Action outcomes, Tick, state, and receipts.
10. Direct prepare/commit is marked transitional and absent from the
    application-facing surface.

## Non-Goals

- Real Edict compiler emission or a production Graft lawpack.
- Provider-v1 or native callback generalization.
- Arbitrary multi-record program semantics.
- Cross-head or cross-worldline atomic Ticks.
- Migration from Graft's git-warp database.
