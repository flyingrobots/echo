<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0026: Durable External-Action Settlement

- **Status:** Accepted
- **Date:** 2026-07-29

## Context

Deterministic application law, external interaction, and model judgment are
different authority categories. A filesystem read, process invocation, network
request, timer, or model call is mechanical but not deterministic over Echo
history. Granting those operations directly to Edict or to the
compiler/provider seam would add ambient authority and make replay consult a
different external world.

Witnessing only the inbound result is insufficient. A crash after an external
system accepts an operation but before Echo records it leaves no durable answer
to whether the operation happened. The outbound request must enter causal
history before an adapter can act.

## Decision

Echo owns a domain-neutral external-action protocol:

```text
Edict decision
    -> canonical request
    -> REQUESTED WAL commit
    -> bounded CLAIMED WAL commit
    -> operation-specific adapter
    -> schema-bound settlement candidate
    -> SETTLED WAL commit
    -> deterministic resumption
```

Edict constructs request values. It does not execute external effects. Echo
records and schedules the boundary crossing. An operation-specific adapter
alone possesses the external credential or host access. The settlement returns
as witnessed ingress.

### Request identity

`ExternalActionRequestV1` commits:

1. the originating `WorldlineId`;
2. a declared operation-family identity;
3. input and settlement schema digests;
4. the requested authority-scope digest;
5. the exact current-world basis digest;
6. a maximum retained-settlement byte budget;
7. a v1 attempt budget fixed to exactly one claim;
8. the canonical input digest; and
9. a named reconciliation-law digest.

The request id is a domain-separated BLAKE3 commitment over those fields.
Changing the worldline, basis, operation, schema, scope, budget, input, or
reconciliation law creates a different request.

The 1 MiB v1 retained-settlement ceiling is an absolute Echo limit, not an
application default. Operation profiles should delegate smaller bounds.

### Claim and authority

The runtime owner installs an `ExternalActionAdapterRegistryV1` containing
operation-, scope-, and adapter-specific bindings. Registry lookup attenuates
that policy into an authorization bound to the exact request id, request basis,
and canonical registry-policy digest. An authorization for another request
cannot be replayed even when operation and scope match. Application code and
Edict receive neither the registry nor the adapter's external credential.

A claim commits the request id, attempt id, zero-based attempt ordinal, adapter
identity, lease or fencing evidence, request-stable idempotency key,
reconciliation law, basis, and registry-policy digest. The attempt identity
binds the registry policy and nonzero lease evidence. The adapter work grant
becomes constructible only after the claim transaction commits.

Protocol v1 admits exactly one claim per request. Recovery of `CLAIMED` is a
reconciliation obligation, not permission to repeat the operation. A retry is
a new deterministic decision and request. This keeps blind re-execution
unrepresentable while later versions establish operation-specific retry laws.

### Settlement

The four terminal settlement kinds are:

- `Succeeded`;
- `Rejected`;
- `Failed`; and
- `OutcomeUnknown`.

`OutcomeUnknown` is not collapsed into failure. It states that the adapter
cannot establish whether the effect occurred. The named reconciliation law and
request-stable idempotency key remain available for a later explicit decision.

An operation profile may retain a reconciliation handle that carries its exact
schema and adapter identity but no external-world capability. The bounded
workspace profile uses such a handle to admit `OutcomeUnknown` after directory
authority disappears. It revalidates the recovered grant and compiler-admitted
request, constructs the canonical operation-specific settlement inside Echo,
and admits it through the ordinary settlement transaction. It cannot observe a
path or construct a successful result.

A settlement binds the exact request, attempt, adapter, basis, settlement
schema, canonical result bytes, result digest, schema-admission evidence, and
external evidence. Echo rejects mismatched claims, stale bases, wrong schemas,
missing schema-admission or external evidence, digest substitution, oversize
results, duplicate settlement records, conflicting settlements, malformed
payloads, and unknown outcome codes.

An adapter that retained its candidate across an acknowledgement loss may
reconcile it against already-settled history. The reconciliation surface
receives no WAL store, transaction context, or claim grant. It validates the
candidate against the recorded request and claim, returns the original
settlement and original commit digest when every field is exact, and appends no
history. A different valid candidate is `ConflictingSettlement`; a malformed
candidate fails ordinary settlement validation before comparison. This is
idempotent result delivery, not permission to execute the adapter again.

The schema-admission evidence is a protocol binding, not a general schema
engine. Each operation profile must define which validator produces that
evidence. The bounded workspace-observation profile supplies the first concrete
validator. It independently accepts exact compiler-owned Core and Target IR,
binds the request to the exact target profile and capability closure, and
validates strictly ordered retained path/content bytes, aggregate bounds,
nonzero evidence, and complete basis evidence before generic settlement
admission.

### WAL ownership

Three transaction kinds own the transitions:

| Transition           | Transaction code | Record code | Frontier              |
| -------------------- | ---------------: | ----------: | --------------------- |
| request admission    |               10 |          29 | `ExternalActionIndex` |
| claim admission      |               11 |          30 | `ExternalActionIndex` |
| settlement admission |               12 |          31 | `ExternalActionIndex` |

All three require `ExternalActionCoordinator` append authority. Each
transaction contains exactly one matching record and advances exactly one
external-action frontier. A frame without its transaction commit is not a
request, claim, or settlement.

Raw WAL builders and raw commit flushes do not carry the coordinator's opaque
capability. A high-level caller supplies only non-causal transaction metadata.
The coordinator derives the next LSN, previous-frame digest, and
previous-commit digest from its checked local WAL continuation. A caller
therefore cannot manufacture coordinator authority or select transaction
coordinates that make a successful append unrecoverable.

The coordinator derives both frontier roots from its canonical lifecycle
index. The root is a domain-separated sparse Merkle commitment keyed by request
id, so insertion order cannot move the reading and one lifecycle update touches
one bounded 256-bit path. One planned mutation computes and retains that path,
then advances the in-memory index only after commit. The request, claim, and
settlement hot paths do not replay prior WAL payloads. Full reconstruction is
reserved for initial or crash recovery. Callers cannot select frontier roots.
Recovery rebuilds the index around every transition and rejects a WAL commit
whose frontier commitment differs.

The high-level APIs return a durable request token, adapter work grant, or
resumable settlement only after the corresponding commit marker flushes. A
commit failure returns no token and poisons that coordinator instance; trusted
local recovery is required before another transition. An uncommitted tail
obstructs further external-action admission until ordinary WAL recovery
resolves it.

### Recovery and replay

Recovery consumes only committed external-action records and reconstructs one
of:

```text
REQUESTED
CLAIMED
SETTLED(SUCCEEDED | REJECTED | FAILED | OUTCOME_UNKNOWN)
```

Missing predecessors, repeated requests or claims, and duplicate or conflicting
settlements obstruct recovery. Settled canonical result bytes are replay input.
Replay does not invoke an adapter. Consulting the current external world again
requires a new program transition and a new request; changing worldline or
basis changes request identity.

The idempotent retry path does not make duplicate WAL history admissible. An
identical candidate is reconciled before any second transaction exists; an
already duplicated settlement record remains a recovery obstruction.

An arbitrary `RecoveryScanReport` produces observation-only lifecycle values.
It cannot mint request-transition tokens, adapter work grants, or resumable
settlement facts. `ExternalActionCoordinatorV1::recover` alone reads one
fallible, coherent local-store snapshot, validates its clean committed history,
and reconstructs those authorities. A crash after request or claim commit can
therefore resume from the durable lifecycle without an ephemeral native return
value.

Filesystem snapshot read or decode failure is an obstruction, never an empty
genesis history. The filesystem recovery witness drops the live store, reopens
its strict filesystem WAL, and recovers the exact settled bytes without
invoking adapter execution.

## Consequences

- The compiler/provider seam remains deterministic and capability-denied.
- Edict may request an operation family without receiving the authority to
  perform it.
- Adapter credentials and host capabilities stay outside Edict and the model.
- Request-before-effect and settlement-before-resumption are type-visible API
  boundaries backed by WAL commits.
- Arbitrary recovered reports remain observation-only; trusted local recovery
  owns transition and replay authority.
- Crash ambiguity has an explicit causal representation.
- Workspace loss after a bounded-observation claim cannot strand the action:
  rootless reconciliation may durably record explicit uncertainty without
  reacquiring read authority.
- Operation-specific idempotency and reconciliation laws remain mandatory;
  Echo does not claim general exactly-once external execution.
- The first capability-rooted read-only workspace adapter is implemented.
  Compiler admission independently reconstructs the request projection from
  Core, enforces both declared and host-owned evaluation budgets, and requires
  enough settlement capacity for every terminal posture. Successful
  settlements must cover exactly the admitted path aperture. Settlement
  admission revalidates the live registry grant, while nonblocking no-follow
  opens refuse special-file substitution without stalling.
- The first capability-rooted mutation adapter applies one validated,
  basis-bound regular-file replacement. Its authority commits the exact path
  aperture and immutable no-follow, single-file, regular-file-only, and
  CI-workflow-exclusion policy. It consumes the bounded-observation basis,
  records the request and claim before mutation, synchronizes a same-directory
  atomic replacement, and records the resulting basis and content identities
  before resumption. Ambiguous attempts are reconciled by bounded
  postcondition observation and never by reapplying the write.
- Process, network, Git, GitHub, timer, and model adapters remain absent.

## Rejected Alternatives

### Callable imports in the provider seam

Rejected. Lowering and independent verification would acquire execution-host
authority and replay could perform new I/O.

### Generic shell, filesystem, or network capabilities

Rejected. These are ambient authority with type names. Adapters must expose
domain-specific operations and validation laws.

### Native stack suspension

Rejected. Waiting is explicit durable protocol state keyed by request identity,
not a serialized host stack or hidden callback continuation.

### Model-owned mutation capability

Rejected. Model output is untrusted data. A model may propose a typed artifact;
deterministic validation and a separately authorized adapter govern mutation.

## Evidence

- `crates/warp-core/src/external_action.rs`
- `crates/warp-core/src/external_action_adapter.rs`
- `crates/warp-core/src/validated_workspace_patch.rs`
- `crates/warp-core/src/causal_wal.rs`
- `crates/warp-core/tests/external_action_protocol_tests.rs`
- `crates/warp-core/tests/bounded_workspace_observation_tests.rs`
- `crates/warp-core/tests/bounded_workspace_patch_tests.rs`
- [External Actions](../topics/ExternalActions.md)
- [WAL](../topics/WAL.md)
- [Runtime Authority](../topics/RuntimeAuthority.md)
- [Security And Authority Boundaries](../topics/security/AuthorityBoundaries.md)
