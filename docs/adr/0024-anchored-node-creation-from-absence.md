<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0024: Anchored-Node Creation as a Separate Executable Program

- **Status:** Accepted
- **Date:** 2026-07-23
- **Amends:** ADR 0023
- **Supersedes:** this ADR's original optional-precondition design

## Context

ADR 0023 established a hook-free executable-operation corridor distinct from
provider-v1. Its first earned program,
`EchoOperationProgramV1::AnchoredNodeAttachmentCompareAndSet`, updates one
typed node's alpha attachment under an exact prior-value digest.

The first version of this ADR widened that program's invocation precondition
from `Hash` to `Option<Hash>`:

- `Some(digest)` meant update;
- `None` meant create the node and attachment from total absence.

That implementation found real missing runtime capabilities. It proved that
creation needs an `UpsertNode` plus `SetAttachment` consequence, node and
attachment occupancy must be checked independently, the node type must be
charged to the write budget, and the two-operation patch must survive WAL
validation and fresh-host recovery.

The widening itself was nevertheless the wrong semantic boundary. Update and
creation differ in every executable dimension:

| Dimension             | Compare-and-set update          | Create if absent                |
| --------------------- | ------------------------------- | ------------------------------- |
| Precondition          | Existing exact attachment value | Node and attachment both absent |
| Patch                 | `SetAttachment`                 | `UpsertNode` + `SetAttachment`  |
| Output slots          | Attachment                      | Node + attachment               |
| Footprint             | Attachment write                | Node + attachment writes        |
| Minimum write bytes   | Attachment type                 | Node type + attachment type     |
| Recovery shape        | One operation                   | Two ordered operations          |
| Occupancy obstruction | Digest mismatch                 | Any occupied target             |
| Result identity input | Prior value digest              | Absence proposition             |

Keeping one program kind while widening its decoder would let already-produced
program bytes and profile identities acquire creation authority merely because
a newer Echo runtime interpreted `Null` differently. An executable content
identity must bind one closed law. Runtime revision is not an admissible hidden
semantic input.

## Decision

### Compare-and-set remains exactly update-only

`EchoOperationProgramV1::AnchoredNodeAttachmentCompareAndSet` retains its
original canonical program bytes and profile coordinates. Its invocation again
requires:

```rust
expected_value_digest: Hash
```

The legacy invocation schema accepts a 32-byte digest and rejects `Null`.
Update admission still requires the node and canonical atom attachment to
exist. Its application-basis proposition, footprint, minimum budget, patch
shape, result schema, result identity calculation, and target profile remain
unchanged.

This compatibility boundary is executable, not rhetorical:

- a unit witness reconstructs the legacy canonical program and invocation
  bytes independently and requires exact equality;
- the legacy result-ID witness reconstructs the pre-ADR-0024 hash input and
  requires exact equality;
- a mutated legacy invocation carrying `Null` is refused.

### Creation is a second earned program

Creation uses:

```rust
EchoOperationProgramV1::AnchoredNodeAttachmentCreateIfAbsent {
    required_node_type,
    required_attachment_type,
    max_replacement_bytes,
}
```

It has its own:

- program-kind coordinate;
- canonical invocation schema with the explicit closed precondition
  `node-and-alpha-attachment-absent/v1`;
- input, result, and obstruction schemas;
- result and obstruction interpretations;
- application-basis schema and proposition domain;
- exact footprint contract;
- target-profile identity;
- result-ID domain;
- canonical two-operation recovery shape.

The interpreter and low-level attachment-algebra profile remain shared. This
is one Echo evaluator with two closed semantic programs, not two runtime
engines and not duplicated application logic.

Package self-validation derives the required profile identities from the
installed program variant. An update invocation cannot be admitted against a
creation package, and a creation invocation cannot be admitted against an
update package.

### Creation observes the full occupancy matrix

Creation's application-basis proposition records node and attachment presence
independently as a closed four-state value:

| Node    | Attachment | Occupancy proposition | Evaluation             |
| ------- | ---------- | --------------------- | ---------------------- |
| Absent  | Absent     | `Absent`              | Create both            |
| Present | Absent     | `NodeOnly`            | `PreconditionMismatch` |
| Absent  | Present    | `AttachmentOnly`      | `PreconditionMismatch` |
| Present | Present    | `NodeAndAttachment`   | `PreconditionMismatch` |

This preserves the distinction between coarse basis corroboration and the
program precondition. A caller can name the honest current occupancy
proposition; Echo independently corroborates it during admission; the creation
program then refuses every occupied state uniformly during private evaluation.
A dishonest claim of total absence against a bare node or orphan attachment is
rejected at admission with `BasisMismatch`.

The evaluator checks node and attachment storage independently. It does not
infer attachment absence from node absence, because `GraphStore` can represent
an orphan attachment. It checks occupancy before validating an occupant's type:
the type of an occupied target is irrelevant to the caller's false vacancy
claim.

### Creation is one atomic semantic rewrite

A successful creation emits exactly:

```text
UpsertNode(required_node_type)
SetAttachment(Atom(required_attachment_type, replacement_bytes))
```

with:

```text
in_slots  = [node, alpha attachment]
out_slots = [node, alpha attachment]
```

The declared and actual footprint reads both locations and writes both
locations. There is no partial-create path that attaches onto an existing bare
node.

The step counter measures deterministic evaluator stages, not the number of
emitted `WarpOp`s. Creation consumes three stages:

1. probe the node;
2. probe the alpha attachment;
3. emit one atomic creation consequence.

The consequence contains two ordered `WarpOp`s but is one semantic rewrite
step. Creation's minimum and exact successful budget for payload length `n` is:

```text
steps       = 3
read_bytes  = 64
write_bytes = 64 + n
```

The 64 write bytes are the 32-byte `NodeRecord.ty` plus the 32-byte attachment
atom type. Compare-and-set remains four stages and charges `32 + n` write bytes.

### Result identity binds absence without perturbing updates

Compare-and-set retains the original result-ID domain, result schema, and raw
prior-digest hash input.

Creation uses a separate result-ID domain and result schema. Its identity input
binds an explicit domain-separated absence proposition rather than an optional
digest or an untagged empty value. Consequently, adding creation does not add a
tag, marker, or alternate interpretation to any update result identity.

### WAL recovery validates the installed semantic shape

Recovery selects the canonical consequence from the exact installed program,
which package and receipt validation bind to the retained commit:

- compare-and-set accepts only one `SetAttachment` operation and an
  attachment-only output list;
- create-if-absent accepts only ordered `UpsertNode`, `SetAttachment`
  operations and node-plus-attachment outputs.

Both profiles additionally require the program-owned attachment type, an atom
value, and a payload no larger than `max_replacement_bytes`. Creation also
requires the program-owned `NodeRecord.ty`. Missing, reversed, partial,
cross-node, cross-profile, wrong-typed, non-atom, or oversized consequences are
refused.

`WorldlineTickPatchV1.warp_id` names the parent worldline's root WARP. Operation
scope comes from the exact `NodeKey` carried by the operations and slots, which
may name a descended WARP instance. Recovery validates the parent root and the
operation scope independently; it does not require those WARP IDs to be equal.
For a descended target, private evaluation walks the validated instance-parent
chain to the root, records every portal attachment as an actual and declared
footprint read, charges one bounded pointer read per portal, and retains the
complete chain in the patch input slots. Recovery requires descendant
consequences to carry attachment inputs reaching the parent root rather than
accepting the target node and attachment alone.

## Scope

This decision closes exactly the:

> single anchored-node-plus-alpha-attachment creation gap

It does not close creation for arbitrary "new facts." It does not provide
multi-node, multi-record, edge, relation, deletion, or general DPO rewrite
semantics.

In particular, Graft's `recordGitWarpImportBatch` operation declares creation
of both `GitWarpImportBatch` and `StructuralBasis`. This ADR does not establish
that one Echo node plus one opaque attachment truthfully represents those two
logical records, and it does not authorize packing them together merely to fit
this primitive. A pinned Graft crossing must determine whether the operation
requires the smallest bounded atomic multi-record program.

This ADR also does not create a real Edict crossing. The current Echo evidence
still uses Echo-side package fixtures. Edict does not yet emit the exact
`ExecutableOperationPackageV1` form, a real Graft lawpack does not yet traverse
the corridor, and a structurally separate target verifier does not yet
corroborate the package.

## Verification Evidence Grade

The evidence grade remains **deterministic self-validation**, not independent
semantic conformance.

The branch's executable-operation pipeline grew from 13 to 24 tests. The
creation matrix and durability witnesses cover:

- success from node-absent plus attachment-absent;
- refusal for node-only occupancy;
- refusal for attachment-only occupancy;
- refusal for node-plus-attachment occupancy;
- uniform `PreconditionMismatch` for a wrong-typed occupied node;
- dishonest absence refusal during admission;
- exact `64 + payload` creation write budget and one-byte-under refusal;
- exact-basis TOCTOU refusal;
- filesystem-WAL commit and fresh-host recovery of the installed package,
  node, attachment, result identity, and typed receipt;
- legacy update missing-state and wrong-node-type behavior.

Focused unit witnesses additionally cover:

- exact legacy update program and invocation canonical bytes;
- exact legacy update result identity;
- rejection of `Null` under the legacy invocation schema;
- installed-program-selected WAL consequences;
- rejection of reversed, partial, wrong-output, cross-node, cross-profile,
  wrong-typed, non-atom, and oversized creation patches;
- descendant-node scope independent of the parent worldline root;
- complete descendant portal-chain footprint, budget, and patch-input binding.

The complete `warp-core` library suite passes with 690 tests, and the focused
pipeline passes all 24 tests. Independent implementation or differential
oracle evidence remains future work.

## Rejected Alternatives

- **Keep `Option<Hash>` in compare-and-set.** Rejected because `None` is a
  strong negative application condition, not "no precondition," and because it
  changes the meaning of existing program and profile identities.
- **Introduce one `/v2` program with `Exact(Hash) | Absent`.** This would be
  semantically honest, but update and creation still have different patch,
  footprint, budget, output, recovery, and result contracts. Two explicit
  program profiles make those differences structural.
- **Infer the program from patch shape during recovery.** Rejected because
  recovery must validate the consequence authorized by the installed semantic
  profile, not let arbitrary retained operations choose their interpreter.
- **Attach onto a pre-existing bare node.** Rejected because it is a third
  semantic program with a different precondition and footprint, and no earned
  caller currently requires it.
- **Represent absence as empty bytes.** Rejected because a present empty atom
  is not absence.
- **Generalize directly to arbitrary multi-record DPO rewrites.** Rejected
  until the pinned Graft RED establishes the smallest missing bounded program.

## Consequences

- Existing compare-and-set program, invocation, and result identities remain
  stable.
- Creation packages and invocations necessarily receive new identities because
  they bind a different executable law.
- Runtime package validation now depends on the selected program profile rather
  than one global set of schema and target constants.
- Creation receipts truthfully report three semantic evaluator steps and
  `64 + payload` write bytes.
- Fresh-host recovery accepts the exact two-operation creation consequence and
  rejects malformed or cross-profile shapes.
- The next convergence task is a pinned external-consumer RED using exact
  Graft schema, manifest, invocation, and real Edict output—without an
  Echo-side handwritten package builder.

## Non-Goals

Unchanged from ADR 0023: no application-specific intrinsic, native callback,
general-purpose VM, arbitrary graph interpreter, external effect execution,
Continuum transport, scheduler batch composition, or production application
cutover is introduced here.

This decision additionally excludes multi-record creation, multi-node
creation, edges, deletion, partial creation, a Graft-specific mapping, Edict
package emission, and replacement of Graft's current transport.

## Evidence Anchors

- `crates/warp-core/src/echo_operation.rs`:
  `EchoOperationProgramV1`, `EchoOperationInvocationV1`,
  `current_application_basis`, `prepare_operation_v1`,
  `operation_result_id`.
- `crates/warp-core/src/trusted_runtime_host.rs`:
  `operation_patch_scope_v1`, `operation_tick_binds_patch_v1`.
- `crates/warp-core/tests/executable_operation_pipeline_tests.rs`: the 24-test
  external-consumer pipeline, including the creation occupancy, budget, TOCTOU,
  and filesystem-WAL witnesses.
- `docs/adr/0023-admitted-executable-operation-packages.md`: the executable
  operation category and program substitution boundary.
- `CHANGELOG.md`: current shipped-surface limits, including the absent real
  Edict and application crossing.
