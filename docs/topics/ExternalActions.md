<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# External Actions

External actions are typed boundary crossings governed by Edict decisions,
Echo history, and separately authorized host adapters. They are not callable
imports, native callbacks, or a second application execution lifecycle.

## Compiler Admission

`admit_edict_external_action_request_v1` accepts exact canonical Core, Target
IR, application input, intent name, and worldline identity. Admission:

1. independently decodes canonical Edict bytes;
2. recomputes the reviewed Core and Target IR digests;
3. requires the Echo Target IR domain and digest-locked target profile;
4. requires the Target IR source-Core binding to match the supplied Core and
   independently proves that the complete Target IR request is the exact
   projection of the supplied Core request;
5. requires exactly one request, its exact result and basis bindings, and zero
   callable target steps;
6. requires the request operation to occur exactly in the digest-locked
   capability closure;
7. evaluates only argument-rooted local, field, record, and constant request
   expressions under the compiler-declared Core budget and Echo ceilings of
   4,096 steps, 4 MiB retained allocation, and 1 MiB output;
8. validates runtime scope, basis, byte, and attempt bounds; and
9. derives the generic `ExternalActionRequestV1`, with the target profile and
   operation identities both committed by its operation identity.

No provider component executes on this route. The provider/lowerer/verifier
seam remains deterministic and capability-denied.

The v1 request profile expects operation input bytes, 32-byte authority and
basis commitments, a retained-settlement bound, and exactly one attempt. A
call expression or callable Target IR step is outside this admission profile.
Admission also requires enough retained-settlement capacity to encode every
terminal posture; a request cannot select a budget that makes rejection,
failure, or ambiguity unrecordable.

## First Adapter Profile

`BoundedWorkspaceObservationAdapterV1` is the first operation-specific
adapter. Runtime configuration supplies:

- one capability-rooted directory;
- an explicit set of permitted relative paths;
- the exact operation, input schema, settlement schema, reconciliation law,
  authority scope, and adapter identities.

Opening the adapter retains directory-relative authority. Observation rejects
empty, absolute, parent-traversing, non-normalized, backslash, escaped,
duplicate, unauthorized, and symlinked paths. Directory components and the
final file are opened without following symlinks. Pre-open and post-open
metadata must both identify a regular file, and the final no-follow open is
nonblocking so a substituted FIFO cannot stall adapter execution. The
request's retained-settlement bound limits aggregate file bytes during reads as
well as the final canonical settlement.

`cap-std` and `cap-fs-ext` are direct `warp-core` dependencies because this
boundary needs an unforgeable directory capability plus component-wise
no-follow opens. Both packages were already pinned in the workspace lockfile;
the adapter does not introduce an ambient path or shell interface.

## Ordering

The adapter accepts only `ExternalActionClaimGrantV1`. That grant exists only
after Echo commits:

```text
REQUESTED
  -> CLAIMED
  -> adapter observation
  -> validated settlement candidate
  -> SETTLED
```

Compiler admission and request recording do not read the configured files.
The operation input is re-bound to its recorded digest before adapter work.
The runtime-owned registry binds the exact operation, scope, and adapter.

## Settlement

A successful settlement retains canonical bytes for every sorted relative
path, each content digest, the complete snapshot basis, and external evidence.
The basis commits every path and byte sequence. A mismatch yields a typed
`Rejected` settlement rather than silently reading a different snapshot.

Path policy, stale basis, and settlement-budget failures are typed
rejections. Definite host I/O failure is `Failed`. Reconciliation may admit
`OutcomeUnknown` with explicit nonzero evidence.

Before generic WAL admission, the operation profile independently validates:

- candidate request, attempt, adapter, schema, basis, and result digest;
- the current registry profile and authority grant against the admitted
  request;
- exact canonical settlement shape;
- outcome/posture agreement;
- exact equality between requested and returned path apertures on success;
- nonempty, strictly sorted, unique, valid relative paths on success;
- per-file content digests and nonzero external evidence;
- complete snapshot-basis equality on success;
- external evidence binding; and
- operation-specific schema-admission evidence.

Malformed or substituted candidates fail before the settlement transaction.

If an adapter loses the acknowledgement after settlement commit, it may submit
the retained candidate to
`reconcile_external_action_settlement_retry`. That function has no store,
transaction context, or claim grant. It returns the exact already-admitted
settlement and its original commit digest without appending history. A
different valid candidate conflicts, while a malformed candidate fails the
same generic validation used for first admission. Reconciliation never makes
adapter execution reachable.

## Recovery And Replay

Recovery reconstructs `Requested`, `Claimed`, or the exact settled outcome
through the generic external-action coordinator. A recovered claim is a
reconciliation obligation, not permission to reread the workspace.

Settled replay returns the canonical bytes retained in the WAL. It does not
open the capability directory again. Removing or mutating source files after
settlement therefore cannot change replay.

Duplicate settlement records still obstruct recovery. Idempotency applies to a
retained candidate reconciled before another WAL transaction, not to duplicated
history.

## Evidence

- `crates/warp-core/src/external_action_adapter.rs`
- `crates/warp-core/tests/bounded_workspace_observation_tests.rs`
- `crates/warp-core/src/external_action.rs`
- `crates/warp-core/tests/external_action_protocol_tests.rs`
- [ADR 0026](../adr/0026-durable-external-action-settlement.md)
- [Runtime Authority](RuntimeAuthority.md)
- [WAL](WAL.md)
