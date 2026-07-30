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
3. requires the Target IR source-Core binding to match the supplied Core;
4. requires exactly one request and zero callable target steps;
5. requires the request operation to occur exactly in the digest-locked
   capability closure;
6. evaluates only local, field, record, and constant request expressions;
7. validates runtime scope, basis, byte, and attempt bounds; and
8. derives the generic `ExternalActionRequestV1`.

No provider component executes on this route. The provider/lowerer/verifier
seam remains deterministic and capability-denied.

The v1 request profile expects operation input bytes, 32-byte authority and
basis commitments, a retained-settlement bound, and exactly one attempt. A
call expression or callable Target IR step is outside this admission profile.

## First Adapter Profile

`BoundedWorkspaceObservationAdapterV1` is the first operation-specific
adapter. Runtime configuration supplies:

- one capability-rooted directory;
- an explicit set of permitted relative paths;
- the exact operation, input schema, settlement schema, reconciliation law,
  authority scope, and adapter identities.

Opening the adapter retains directory-relative authority. Observation rejects
empty, absolute, parent-traversing, non-normalized, backslash, escaped,
unauthorized, and symlinked paths. Directory components and the final file are
opened without following symlinks. Only regular files are readable.

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
- exact canonical settlement shape;
- outcome/posture agreement;
- per-file content digests;
- complete snapshot-basis equality on success;
- external evidence binding; and
- operation-specific schema-admission evidence.

Malformed or substituted candidates fail before the settlement transaction.

## Recovery And Replay

Recovery reconstructs `Requested`, `Claimed`, or the exact settled outcome
through the generic external-action coordinator. A recovered claim is a
reconciliation obligation, not permission to reread the workspace.

Settled replay returns the canonical bytes retained in the WAL. It does not
open the capability directory again. Removing or mutating source files after
settlement therefore cannot change replay.

## Evidence

- `crates/warp-core/src/external_action_adapter.rs`
- `crates/warp-core/tests/bounded_workspace_observation_tests.rs`
- `crates/warp-core/src/external_action.rs`
- `crates/warp-core/tests/external_action_protocol_tests.rs`
- [ADR 0026](../adr/0026-durable-external-action-settlement.md)
- [Runtime Authority](RuntimeAuthority.md)
- [WAL](WAL.md)
