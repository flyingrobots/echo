<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Contract Inverse Admission

Contract inverse admission is Echo's generic boundary for undo-as-history.
Echo does not infer an inverse from a replay patch and does not remove an old
transition. An installed application contract derives an ordinary mutation
intent from retained causal evidence, and Echo admits that intent through the
same durable path as any other application mutation.

## Authority Split

Echo owns:

- exact causal receipt identity;
- witnessed canonical submission material;
- installed artifact verification;
- current worldline frontier validation;
- WAL-backed submission acknowledgement;
- causal parent retention;
- scheduler admission, ordering, and receipts.

The installed contract owns:

- whether an operation is invertible;
- how the original operation maps to the current domain frontier;
- which retained fragments are required;
- how application policy changes inverse behavior;
- the canonical operation id and variables of the inverse intent;
- typed domain obstruction when an honest inverse cannot be produced.

Neither side owns the other's job. Echo must not interpret `WarpOp` replay
material as domain undo. A contract callback must not stage ingress, tick the
scheduler, mutate runtime state, or mint Echo receipts.

## Installed Boundary

An `InstalledContractPackage` may register a `ContractInverseHandler` for a
mutation operation installed by that same package. Package preparation rejects:

- duplicate inverse handlers;
- handlers for unknown operations;
- handlers for query operations;
- handlers whose target mutation is not installed by the package.

That inverse-handler boundary is currently the legacy generated-contract
family. Provider-native invocation evidence is preserved as its own proposition
and is rejected with the typed `ProviderTargetUnsupported` obstruction. Echo
does not reinterpret provider evidence as a legacy package coordinate, infer an
inverse from Target IR, or invoke a provider callback under a law that was never
admitted. A later provider inverse crossing must supply and verify its own exact
law and evidence.

The handler receives `ContractInverseContext`, which contains:

- the exact target `CausalTickReceiptRef`;
- the target witnessed submission id;
- the contract evidence retained on the target transition;
- the original intent kind, operation id, and canonical variables;
- the original and current ingress targets;
- the validated current worldline frontier tick;
- the exact receipt set for the current frontier commit;
- canonical application policy bytes;
- read-only runtime and provenance evidence.

The handler returns one `ContractInverseIntent` or a
`ContractInverseHandlerError`. The returned operation must be a mutation owned
by the same installed package as the target transition.

## Admission Flow

`TrustedRuntimeApp::submit_contract_inverse_with_runtime_wal_ack` performs the
following steps:

1. Require an active runtime WAL.
2. Resolve the exact target receipt from the recovered receipt-correlation set.
3. Resolve the canonical witnessed submission named by that correlation.
4. Decode the original generated mutation operation and variables.
5. Require mutation evidence on the target transition.
6. Require an exact match with the currently installed contract artifact.
7. Require the requested current worldline and frontier tick to still match.
8. Resolve every retained receipt in the current provenance-tip commit.
9. Invoke the installed read-only inverse law.
10. Validate that the produced mutation belongs to the same package.
11. Build a normal local intent citing the target receipt with the typed
    `ContractInverseTarget` relation and citing the current-basis receipt set as
    ordinary causal dependencies.
12. Commit the normal submission-acceptance transaction before returning.

The returned submission is only witnessed ingress history. It is staged and
ticked through the ordinary trusted-host path. Its eventual receipt retains the
target and current-basis receipts in `causal_parent_receipts`, while the
witnessed ingress envelope preserves which receipt was the inverse target.
Recovery rebuilds both parent and reverse-child indexes from durable evidence.
Including relation roles and the current basis in ingress identity prevents
semantically different derivations from collapsing into one submission.
Although the envelope representation and codec preserve the typed relation,
ordinary app and runtime submission reject `ContractInverseTarget`. Only the
validated contract-inverse admission path can authorize that role, so retained
history cannot misclassify an arbitrary app-authored intent as a
contract-defined inverse.

## History Projection

`TrustedRuntimeApp::contract_inverse_derivation` resolves an admitted receipt
through retained receipt correlation and its witnessed ingress envelope. It
returns:

- the admitted inverse receipt;
- the exact target receipt selected for inversion;
- the canonical current-basis receipt set used at admission.

An ordinary non-inverse receipt returns `Ok(None)`. Missing inverse receipt,
witnessed submission, target receipt, or basis receipt evidence returns a typed
`ContractInverseHistoryObstruction`. Multiple retained inverse-target roles are
also an obstruction. The query never consults or repairs a process-local
request map.

## Obstruction Is Truth

Echo obstructs before submission when:

- an ordinary submission claims the reserved contract-inverse target role;
- the exact target receipt is unavailable;
- the target receipt records a rejected or otherwise non-applied outcome;
- the target witnessed envelope is unavailable or malformed;
- target contract evidence is absent or inconsistent;
- the target carries provider-native evidence but no provider inverse law is
  admitted;
- the matching artifact or inverse handler is not installed;
- the installed artifact differs from the artifact retained on the target;
- the application's current frontier is stale;
- current provenance or its retained receipt basis cannot be resolved;
- the inverse law emits an uninstalled or cross-package mutation;
- the inverse variables cannot be encoded.

The contract can separately report:

- unavailable inverse fragments;
- an unmappable causal span;
- compressed history that requires rehydration;
- another stable application-defined obstruction code.

No obstruction creates a submission, advances a worldline, or alters the
target transition.

## Durability And Restart

Receipt correlations and witnessed submission envelopes are reconstructed from
the runtime WAL. Both inverse admission and inverse-derivation observation
therefore resolve the same target after a host restart without a process-local
undo map. An in-memory index may accelerate that traversal, but it is never
authority and may be discarded and rebuilt at any time.

The installed executable contract package is host configuration and must be
reinstalled after restart. For the currently supported legacy inverse path,
Echo compares its full retained evidence identity to the target transition
before invoking its inverse law. Reinstalling a newer or otherwise different
artifact produces `ContractVersionMismatch` rather than silently applying a
different law to old history. Reinstalling a provider package makes future
provider mutation dispatch possible but does not create provider inverse
authority.

## Sequence Semantics

The current boundary returns one ordinary mutation intent for one inverse
request. This is deliberate: Echo does not claim atomicity for a partially
submitted list of application operations.

A contract that needs atomic sequence undo should expose one mutation whose
canonical variables name the ordered target sequence and whose normal handler
returns complete, partial, conflict, or obstructed domain posture. A caller may
also issue individual inverse requests serially, but each later request must use
the frontier produced by the previous admitted transition. Echo does not treat
several independent submissions as an atomic inverse transaction.

## Jim Consequence

Jim undo and redo should retain exact receipt references, ask Echo for a
contract-defined inverse, and render the resulting causal history. Process-local
undo stacks may remain disposable navigation projections, but they must be
reconstructed from Echo history and must never decide what can be undone.
