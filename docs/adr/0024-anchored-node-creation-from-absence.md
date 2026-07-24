<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0024: Anchored-Node Creation as Compare-and-Set From Absence

- **Status:** Accepted
- **Date:** 2026-07-23
- **Amends:** ADR 0023

## Context

ADR 0023 added Echo's first hook-free executable-operation program,
`EchoOperationProgramV1::AnchoredNodeAttachmentCompareAndSet`: an admitted
program that anchors one typed node, compares the digest of its typed alpha
attachment against an invocation-supplied precondition, and replaces that
attachment. Both layers that corroborate this precondition -- the coarse
admission-time application-basis check (`current_application_basis`) and the
fine-grained evaluation-time check inside `prepare_operation_v1` -- currently
require the node and its attachment to already exist. A missing node or
missing attachment is an unconditional refusal
(`NodeMissing`/`AttachmentMissing`/`"application-basis node is unavailable"`),
never a path to success.

This was sufficient for ADR 0023's accepted first vertical, Jedit's
`ReplaceRange`, because Jedit's buffer nodes are created through a separate,
native `CreateBufferWorldline` path outside the executable-operation corridor
(`docs/plans/2026-07-18-jim-edict-echo-executable-warp-semantics.md`, section
1.1). `ReplaceRange` only ever edits a node that already exists; it never
needed to create one lawfully.

A second, independent real-world caller does not share that shape. Its writes
are new facts -- new records, keyed by identities that have never existed
before -- not edits to a long-lived node whose creation happened through some
other channel. That caller has no equivalent native creation path to borrow;
if it cannot create an anchored node and its attachment through an admitted
Edict-authored operation, it cannot create one lawfully at all, under this
corridor, full stop.

This is a corridor gap, not an application gap. It would recur for any future
caller whose writes are creation-shaped, regardless of which application
authors the lawpack. Fixing it once, generically, in the one program kind
that already exists is smaller than asking every future creation-shaped
caller to solve the same problem, and stays inside the DPO framing ADR 0023
already declared for this program family (`L`/`K`/`R` spans with positive and
negative application conditions): a creation rule is simply a span whose `L`
is empty for the created element. It is not the general "Cyber Kitten"
graph-rewrite runtime ADR 0023 explicitly declined to build before the
operation seam worked ("Rejected Alternatives": _"Build the general Cyber
Kitten runtime before the operation seam works"_). That decision stands;
this widens one earned primitive, it does not add a second engine.

## Decision

### The precondition becomes optional, not a second program kind

`EchoOperationInvocationV1`'s per-invocation precondition widens from a
required digest to an optional one: `expected_value_digest: Option<Hash>`.

- `Some(digest)` is exactly today's update precondition, byte-for-byte
  unchanged: the node and its typed alpha attachment must already exist, and
  the attachment's current digest must equal `digest`.
- `None` is a new, disjoint precondition: the caller asserts that the node
  and its typed alpha attachment are both **entirely absent**. If reality
  agrees, Echo creates both atomically in one patch. If anything already
  exists there -- the node, its attachment, or both -- the invocation
  refuses.

This is a single-field widening of the existing primitive, not a new program
kind. `EchoOperationProgramV1::AnchoredNodeAttachmentCompareAndSet` and its
wire `PROGRAM_KIND` constant are unchanged; only the invocation-level
precondition's cardinality changes. No new obstruction kind is introduced:
"something already exists where absence was claimed" is reported as
`PreconditionMismatch`, the same category already used for "the existing
value doesn't match what was claimed" -- both are the same concept, a
declared expectation about current state that reality contradicts.

### Creation is atomic and total, never partial

A node that exists without its alpha attachment does **not** satisfy the
`None` precondition. Creation succeeds only when both the node and its
attachment are absent, and it creates both together in one patch. There is no
path in this primitive that attaches an alpha value onto a pre-existing bare
node. A future caller that genuinely needs that intermediate state is a
separate, later decision -- nothing here forecloses it, and nothing here
builds it speculatively.

This keeps the declared/actual footprint contract exact rather than
conditional: `anchored_node_footprint` declares node-write
(`Footprint::n_write`) if and only if the invocation's precondition is
`None`, and the evaluator only ever performs `WarpOp::UpsertNode` on exactly
that same branch. Declared and actual footprints stay bit-identical in both
the update and the create case, preserving the existing
`actual_footprint != declared_footprint -> FootprintViolation` equality
check without weakening it to a containment check.

The created node's `NodeRecord.ty` is exactly the package-declared
`required_node_type` -- the same static type already governing the update
path. An admitted package cannot create a node of an undeclared type through
this primitive. `WarpOp::UpsertNode` already existed in `tick_patch.rs`'s
op vocabulary and is reused as-is; no new op kind was needed, matching the
finding in the 2026-07-18 plan doc that "the new evaluator does not need a
second commit format."

### The admission-time freshness check widens symmetrically

`current_application_basis` -- the independent, coarser corroboration
`admit_invocation_v1` performs before evaluation even begins -- widens the
same way. A node that does not exist, or that exists without its attachment,
now corroborates as a new, domain-separated **absent** application-basis
proposition
(`echo_operation_anchored_node_absent_application_basis_v1`), rather than
failing admission outright. The invocation's claimed application basis is
then judged by the same equality check `admit_invocation_v1` already used --
a claim of absence matches only real absence, exactly as a claim of a value
matches only that exact value.

The absent proposition is hashed under its own domain constant
(`APPLICATION_BASIS_ABSENT_DOMAIN`), separate from
`APPLICATION_BASIS_VALUE_DOMAIN`. It cannot collide with any present value's
digest, including a legitimately empty one -- absence and "an atom that
happens to be zero bytes long" are different propositions and must never be
represented by the same bytes.

The atomic-or-refuse distinction between "node absent" and "node exists
without its attachment" belongs only to the finer evaluation-time check in
`prepare_operation_v1`, not to this coarse admission check, matching the
existing division of labor: admission already never checked node _type_
either, leaving that to evaluation alone.

### No application awareness enters Echo

`required_node_type`, `required_attachment_type`, and every wire schema
constant this primitive uses remain exactly as generic as before: runtime
`TypeId` values and version-scoped profile strings, never an application,
package, or coordinate name compiled into Echo's source. This decision adds
no knowledge of any specific caller -- not Jedit, not any other application
-- to `warp-core`. The primitive still does not know what it is being used
to build.

## Verification Evidence Grade

Per ADR 0023's evidence-grade table, this change's evidence is **deterministic
self-validation**: `crates/warp-core/tests/executable_operation_pipeline_tests.rs`
gained six tests exercising the new behavior end to end through the real
`TrustedRuntimeHost` (admission, private evaluation, commit, and post-commit
store readback) --

- `create_from_absence_commits_one_new_node_and_attachment_patch`: a genuinely
  absent node and attachment are created atomically, with the exact declared
  budget consumed and the exact node/attachment values readable afterward.
- `create_from_absence_refuses_when_the_node_already_exists`: a `None`
  precondition against a real, existing node refuses with
  `PreconditionMismatch` and leaves the existing attachment untouched --
  exercised with a _correct_ evaluation basis (so the coarser admission-time
  check passes), isolating the finer evaluation-time check as the thing under
  test.
- `update_precondition_still_refuses_when_the_node_is_absent`: a `Some(...)`
  (update-shaped) precondition against a genuinely absent node still refuses
  with `NodeMissing`, unchanged -- proving the widening did not weaken the
  existing update path's requirements.
- `create_from_absence_refuses_when_the_node_exists_with_the_wrong_type`: a
  node present with a different `NodeRecord.ty` than the installed package
  declares refuses with `NodeTypeMismatch`, not a generic precondition
  failure, even though admission's coarser check (which never inspects node
  type) still admits it.
- `create_from_absence_refuses_when_the_node_exists_without_its_attachment`: a
  node present with the correct type but no alpha attachment refuses with
  `PreconditionMismatch` -- creation is atomic over both slots or it refuses,
  with no path that attaches onto a pre-existing bare node.
- `create_from_absence_cannot_commit_after_its_parent_basis_changes`: the
  existing basis-changed TOCTOU protection covers a prepared create-from-
  absence patch through the same generic exact-basis commit check already
  proven for updates, not a create-specific carve-out.

All 13 pre-existing tests in that file pass unmodified in behavior (only
mechanically rewrapped in `Some(...)` at call sites, never changed in intent).
This evidences that `EchoOperationInvocationV1`'s canonical byte encoding and
`prepare_operation_v1`'s/`current_application_basis`'s update-path _behavior_
are unchanged for `Some(...)`. It does **not** evidence that every internal
digest is byte-identical: `operation_result_id`'s hash input now goes through
`hash_optional_id`, which adds a one-byte discriminant tag ahead of the
32-byte digest for _both_ `Some` and `None`, not only `None`. This changes
`EchoOperationResultIdV1` -- and everything chained from it, including
`preparation_id` and receipt identities -- for the update path too, not only
the new create path. This is judged acceptable, not a correctness defect: per
the CHANGELOG.md entry for the executable-operation-runtime slice, no real
committed result or receipt identity exists yet for this corridor to break.
A future change that needs `operation_result_id`'s update-path output to stay
byte-identical across a wire revision is a separate, explicit decision, not
something this ADR's "unchanged" claim should be read to already cover.

The full `warp-core` suite (81 test binaries, 686 library tests, doctests
included) passes with this change; `cargo clippy --all-targets` reports the
identical pre-existing error set before and after (57 errors, all unrelated
to this change, none introduced by it), independently confirmed against a
clean `origin/main` worktree by exact file:line location, not only by
message text -- the sole difference is one pre-existing lint's line number
shifting by this change's own insertion offset.

This grade is not independently implemented conformance evidence -- there is
no second, separately-implemented evaluator checked against this one. That
grade remains reserved for differential-oracle work of the kind ADR 0023
described for `ReplaceRange`.

## Jurisdiction

Unchanged from ADR 0023. This decision touches only Echo's target
implementation (the program/invocation wire shapes and their canonical
encoding) and Echo's runtime (admission's `current_application_basis` and
evaluation's `prepare_operation_v1`). It grants no new authority to any
application, host, or capability, and does not touch Edict, a lawpack, or any
generated client.

## Rejected Alternatives

- **Add a second named program kind** (e.g. `AnchoredNodeAttachmentCreate`)
  duplicating most of the compare-and-set logic. Rejected: a single optional
  field cleanly unifies both preconditions under one primitive with one wire
  schema, one footprint contract, and one evaluator match arm, rather than
  forking the primitive's own logic in two.
- **Allow partial creation** -- setting an attachment onto a node that already
  exists without one -- under a `None` precondition. Rejected: it breaks the
  atomic, exact declared/actual footprint equality this decision otherwise
  preserves, introduces a third observable state, and no known caller needs
  it yet. A future decision can add it deliberately if one does.
- **Represent absence as zero-length bytes** through the existing
  `echo_operation_atom_value_digest_v1` path instead of a domain-separated
  proposition. Rejected: it would make "nothing exists here" indistinguishable
  from "an atom that happens to be empty," which is a real, legitimate,
  different state under this primitive's own type system.
- **Generalize now into a full DPO `L`/`K`/`R` interpreter** that accepts
  arbitrary structural rewrites. Rejected for the same reason ADR 0023
  rejected it for the first vertical: this widening earns exactly the
  creation case a second real caller needed, proven end to end, rather than
  building unproven generality ahead of a second earned witness.

## Consequences

- `EchoOperationInvocationV1`'s canonical CBOR encoding changes: the
  `expected_value_digest` field is now `Bytes | Null` instead of always
  `Bytes`. This changes `EchoOperationInvocationV1::identity()`'s output for
  any invocation bytes built before this change. This is judged acceptable
  because, per `CHANGELOG.md`'s entry for the executable-operation-runtime
  slice, this corridor does not yet include Jedit's `ReplaceRange` lawpack,
  scheduler batch composition, or any production cutover -- there is no real
  committed invocation whose identity this could retroactively break.
- `current_application_basis` widens symmetrically; a create-from-absence
  invocation's freshness is corroborated exactly like an update's, just
  against a different current-state predicate (absence vs. a specific
  value), through the same equality check. No local budget guard was added
  for absence corroboration's read cost: `admit_invocation_v1`'s existing
  `installed.program.minimum_budget().fits_within(invocation.delegated_budget)`
  check already requires `read_bytes >= 64` -- the exact cost of probing an
  absent node and attachment -- before `current_application_basis` is ever
  called, so a second, local `< 64` guard at that point would be unreachable
  dead code, not defense in depth.
- The update path's _behavior_ -- which preconditions succeed, which refuse,
  and with which obstruction -- is unchanged, evidenced by the pre-existing
  test suite passing without behavioral modification. Its digests are not
  entirely unchanged, though: `operation_result_id` now hashes
  `previous_value_digest` through `hash_optional_id`, which prepends a
  discriminant tag for both `Some` and `None`. `EchoOperationResultIdV1` (and
  everything chained from it) therefore differs from what it would have been
  under the prior code, for the update path as well as the new create path.
  `EchoOperationInvocationV1`'s own canonical bytes are unaffected for
  `Some(...)` -- only this one internal hash's input layout changed. Judged
  acceptable for the same no-real-cutover-yet reason as the invocation
  encoding change above; a future decision that needs this specific digest to
  stay byte-identical across the change is separate and not yet made.
- Echo still creates nodes of only the package-declared, static
  `required_node_type`; it still contains no application-specific intrinsic
  and no knowledge of what any caller is building.

## Non-Goals

Everything ADR 0023 already placed out of scope remains out of scope here:
Cyber Kitten syntax or runtime, `TextWindow` or other optic migration,
observer routing, durable child lanes or child-local ticks, wormholes,
holograms, Continuum transport, external effect execution, arbitrary native
plugins or a general-purpose VM, broad application authorization, and
fork/braid/settlement migration. This decision additionally does not add:
multi-node or multi-edge creation, deletion of an existing node, partial
creation onto a pre-existing bare node, or any second program kind.

## Evidence Anchors

- `crates/warp-core/src/echo_operation.rs`: `EchoOperationProgramV1`,
  `EchoOperationInvocationV1`, `prepare_operation_v1`'s
  `AnchoredNodeAttachmentCompareAndSet` match arm, `current_application_basis`,
  `anchored_node_footprint`, `echo_operation_anchored_node_absent_application_basis_v1`.
- `crates/warp-core/tests/executable_operation_pipeline_tests.rs`: the three
  new tests named above, and the 13 pre-existing tests they sit beside.
- `docs/adr/0023-admitted-executable-operation-packages.md`: the corridor and
  primitive this decision amends.
- `docs/plans/2026-07-18-jim-edict-echo-executable-warp-semantics.md`, section
  1.1: `CreateBufferWorldline` as Jedit's out-of-corridor node-creation path,
  the reason this gap did not block ADR 0023's first vertical.
- `CHANGELOG.md`: the executable-operation-runtime slice entry establishing
  that no real cutover has landed yet.
