<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Generated Rule Authorship

Product and application rules are authored in contract languages. Hand-written
Rust rule registration is not a supported product authoring surface. The first
Edict provider-v1 compatibility closure now reaches provider-native scheduling,
receipts, and WAL recovery. Its semantic implementation remains host-bound.
Echo now also has the first generic, hook-free executable-operation runtime
slice. The standalone Hello Echo application emits its package through Edict;
no Jedit consumer emits one yet. Wesley packaging and the generated
bounded-read corridor remain incomplete and must not borrow evidence from
either mutation path.

## One current version and explicit migration

Each data structure, wire format, and implementation has exactly one supported
version in the main crates. A replacement updates the current definition and
its producers and consumers together. Main crates must not retain historical
definitions, decoders, execution paths, compatibility aliases or wrappers,
feature-gated old implementations, or runtime dispatch between old and new
versions. Distinct current concepts may have distinct types; historical
versions of the same concept may not coexist in the main crates.

Old versions may exist only in separate upgrade or migration tools. If future
persisted data needs conversion, those tools must explicitly convert and
validate it before the current runtime consumes it. This policy does not claim
that such tools already exist. Version labels on the one current format do
not authorize retaining another version in the runtime.

The user confirmed on 2026-10-19 that there are no existing saved histories to
preserve. This decision supersedes the V1-preservation and dual-version
installation/recovery direction in proposal #889, earlier compatibility
promises, and any historical decision that would require old and new versions
of the same structure to coexist in a main crate. The implementation sections
below describe the branch's current behavior, including its still-present
V1/V2 duplication; that duplication does not satisfy this policy. A completed
authored-effect bridge must replace it with one current format and update
the compiler, runtime, tests, and consumers together.

## Current Implementation

Wesley currently emits raw `RewriteRule` builders plus generated operation,
handler, observer, and footprint helpers. Its integration fixture enables
`native_rule_bootstrap` and calls `Engine::register_rule` directly. Wesley does
not yet package that output. It does not emit an `InstalledContractPackage` or
exercise package verification.

The Edict provider path authenticates exact authored semantic source and
generation settings, emits canonical provider artifacts, lowers the supported
mutation closure into `echo.span-ir/v1` Target IR, and checks that crossing
through a structurally separate verifier path. That separation is not, by
itself, an independently implemented verifier. The path publishes checked
lowerer and verifier components inside a digest-locked provider package with
exact provenance and non-authoritative review evidence. The lowerer also emits
a requested-only Rust helper that binds the exact Target IR, bundle
propositions, profiles, schemas, ABI versions,
footprint obligation, obstruction mapping, and semantic operation identity
through a pure descriptor preflight. The generated-artifact profile owns the
`le-binary-v1` value codec; the helper implements distinct typed `Id`, `Input`,
and `Output` encoders/decoders, rejects malformed or trailing bytes, and packs
the typed input into canonical EINT v1. The EINT `vars` payload remains opaque,
codec-owned bytes rather than a universal canonical-CBOR value.

That provider package and descriptor are compiler/publication artifacts. They
are not an Echo registry entry, installed package, submitted intent, execution
receipt, or observation. The descriptor exposes a borrowed, provider-generic
registry and can combine its generated matcher with one explicitly
identity-bound host implementation to produce an opaque, non-installing
provider package proposal. Proposal preflight fails closed across the complete
operation, Target IR, bundle, profile, schema, codec, obstruction, ABI, helper
API, and footprint claims. Matching callback claims are cross-binding evidence,
not proof that arbitrary callback code implements the claimed semantics.
The result is a non-installing provider package proposal.

`TrustedRuntimeHost` can now consume that proposal under an independently
constructed `ProviderContractAdmissionPolicyV1`. Exact agreement on the
host-owned occurrence claim and complete provider registry yields an opaque
`AdmittedProviderContractPackageV1`; semantic and release mismatches remain
distinct typed failures. This is claim admission, not package-byte admission:
the crossing performs no package loading or rehashing. It also performs no
installation, registry mutation, callback invocation, scheduling, receipt, or
observation. Echo's existing `InstalledContractPackage` verification and
`Engine::register_contract_package` remain the Wesley compatibility path.
`echo-wesley-gen` now consumes the admitted token with an independently produced
`DigestAdmittedProviderPackageV1`; exact provider-coordinate and strict
lowercase SHA-256 package-root agreement yields an opaque
`DigestCorroboratedProviderContractPackageV1`. This corroborates occurrence
only, not registry semantics or callback behavior. A proof-owning
`echo-wesley-gen` adapter now consumes that token through `warp-core`'s sealed
runtime-owner installer port. `TrustedRuntimeHost` creates a distinct owned
provider record and atomically updates provider package, root, operation, and
shared scheduler-rule indexes. Before those updates, Echo subjects the package
reference, operation coordinate, and Target IR evidence fields to the same pure
structural validation used by WAL recovery. It invokes no callback and invents
no Wesley/GraphQL metadata or legacy installed-contract evidence.
Generated code cannot install itself. The application handle exposes no
installation surface.

An installed provider mutation can now enter Echo through
`TrustedRuntimeHost::admit_provider_contract_submission_v1(...)` after ordinary
witnessed submission. The outer intent kind must be exactly canonical EINT v1,
and the encoded operation id must resolve to the installed provider mutation
before staging. Echo uses the existing scheduler and retains a distinct
`ProviderV1` invocation-evidence proposition binding the installed package id,
exact package reference, operation, Target IR, and scheduler rule. A provider
outcome is applied only when the exact bound provider rule appears in the tick
receipt; a system acknowledgement cannot satisfy that proposition. Provider
evidence survives the tagged WAL and fresh-host recovery after the same package
is reinstalled as host configuration, without fabricating legacy contract
coordinates or rerunning work.

`native_rule_bootstrap` is a Cargo feature gate and repository policy boundary.
Default builds omit raw rule constructors and public registration methods, but
a Rust dependency consumer can explicitly enable the feature.

It is not an access-control or security seal. Echo product and adapter code must
not use it as an application authoring escape hatch.

## Execution Corridors

### Bounded compiler-package computation

`edict_pure::evaluate` interprets exact compiler-produced packages under an
independently selected trusted-host pin and bounded input, work, storage and
output limits. This is ordinary computation, not package admission, graph
mutation, an Action, a Tick, a receipt or recovery evidence.

The interpreter supports `core.integer.add<U32/U64>`. It validates both operands
against the declared unsigned width and checks the sum before copying the
result through the ordinary resource meter. Overflow and invalid operands
refuse rather than wrap. The literal-boundary witness needs eight work units
and three 64-byte value cells; one fewer work unit or storage byte refuses.
The public compiler fixture and arithmetic meter test in
`edict_unsigned_addition_tests` and `edict_pure::evaluate` cover these boundaries.

Native lowerer and verifier source-function judgments independently require
exact same-width operands and a dominating no-overflow proof. Checked conjunctive
prefixes can bound later terms; facts cannot escape negation or flow backward.
Bounded-read relations reuse this established proof and require exactly
matching declared and operand widths. New addition calls require this judgment
even when the legacy Core function table is absent. Both implementations bound proof traversal and comparisons
using their existing work ceilings. Rebound native tests exercise these checks;
they do not establish public compiler acceptance.

Runtime helper validation also proves addition within its fresh frame using
literal and nested-sum bounds. Other terms retain their full unsigned width,
so zero identity is total and an unbounded parameter plus one refuses. Caller
constraints do not cross into a helper frame.

The retained public compiler fixtures cover a legacy-producer execution control
and a strict SourceFunctions publication containing an authored arithmetic helper.
The accepted public verifier report binds the unmodified package bytes; repeated
public compilation preserves both package and report identity.
Checked provider components are promoted from two byte-identical designated
builds, and the package manifest binds their exact digests. Component presence,
runtime support and native judgments alone do not authorize a package or grant
application mutation authority.

### Provider-v1 compatibility corridor

The currently implemented Edict mutation corridor is:

```text
Edict source
-> reviewed mutation Target IR
-> generated Rust mutation handler, codecs, footprint, registry, and package metadata
-> opaque provider package proposal with explicit host binding
-> Echo-owned exact proposal-claim admission
-> exact package corroboration
-> provider-native installation
-> existing atomic engine rule and operation indexes
-> exact EINT-kind and installed-operation admission
-> scheduler-owned execution
-> receipt / WAL evidence carrying package, operation, Target IR, and rule identity
```

This is provider-v1 callback-shaped compatibility infrastructure, not the
required authoring path for newly authored executable operations. Edict already
emits a digest-locked provider publication package, codec-bound mutation client,
borrowed registry, and fail-closed package proposal. Echo admits the exact
proposal claim under a separate trusted-host policy; `echo-wesley-gen`
corroborates that token with separately admitted exact package bytes and
consumes the proof into a provider-native installed record with atomic
Echo-owned indexes. Echo admits and dispatches that installed mutation through
the shared scheduler with provider-specific receipt and WAL evidence. Those
identities prove exact wiring, not that the ambient callback implements the
authored meaning.

Wesley's direct rule/bootstrap and bounded-read work remains a separate
compatibility corridor. Wesley still needs a package emitter and its own
installed generated-pack witness. The Edict mutation proposal intentionally
rejects `Query`; that refusal does not turn a read into a mutation or eliminate
the observer/optic crossing.

### Executable-operation corridor

[ADR 0023](../adr/0023-admitted-executable-operation-packages.md) governs the
operation-oriented corridor required for newly authored application mutations.
The currently implemented generic runtime slice is:

```text
exact canonical ExecutableOperationPackage bytes
-> Echo-owned package admission under a separate policy
-> installed data-only EchoOperationProgram
-> installed compiler-owned application-result projection
-> exact basis-bearing projected invocation with canonical application input
-> canonical Action submission retained before acknowledgement
-> Echo-owned invocation admission under authority and delegated budget
-> ordinary head-inbox staging and scheduler selection at one exact basis
-> bounded private evaluation with recorded actual footprint during Tick construction
-> exact typed application-result bytes and domain-separated identity
-> deterministic composition of independent Actions into one Tick consequence
-> typed committed, obstructed, or footprint-conflict outcome for every Action
-> decided Tick WAL retention before publication and callback-free result recovery
```

The first two programs are an anchored typed-node alpha-attachment
compare-and-set and a separate create-if-absent profile. Compare-and-set keeps
its original update-only bytes and identities. Creation has distinct
invocation, schema, footprint, application-basis, result, recovery, and target
profiles; it creates exactly one node and its alpha attachment only when both
locations are absent. These are generic vertical witnesses, not Jedit
`ReplaceRange`, a Graft multi-record mutation, a rope intrinsic, or a
general-purpose virtual machine. The package binds Edict source, canonical
meaning, Core, Target IR, application-schema, and lawpack identities as opaque
substitution evidence. The checked provider lowerer obtains those identities
from a real Edict compiler crossing, and its structurally separate verifier
reconstructs the semantic relation. Target IR carries target obstruction
coordinates; source-lawpack failure names reach Echo only through the
independently checked adapter mapping.

The first external compiler-to-runtime singleton witness is:

```text
standalone application-owned Edict source
-> exact Core, lawpack, exports, target adapter, target configuration, and Target IR
-> compiler-produced ExecutableOperationPackage
-> structurally separate accepted verification report
-> exact result-projection binding and generic typed-input binding
-> projected invocation retaining exact canonical application input
-> accepted-submission WAL commit before acknowledgement
-> fresh-host recovery of the exact installed package and pending Action
-> scheduler-owned private evaluation of the recovered Action and one singleton Tick
-> exact compiler-declared result bytes in the committed outcome and Receipt
-> decided-Tick WAL commit before publication
-> second fresh-host package, Action, Tick, state, result, outcome, and Receipt recovery
-> typed duplicate-create obstruction with no hidden mutation or application result
-> refusal to recover against a mutated initial state
```

#### Action validity across ticks

**Decision 2026-10-14 ([#837](https://github.com/flyingrobots/echo/issues/837)),
superseding in part [ADR 0023](../adr/0023-admitted-executable-operation-packages.md)'s
exact-basis requirements and [ADR 0025](../adr/0025-scheduler-owned-executable-operation-actions.md)'s
shared parent basis, for Actions that meet its conditions.**

What is implemented today is a conservative subset:

- **Programs.** Compare-and-set and create-if-absent Actions, with the target
  and any observed nodes in the worldline's root warp, and any observation
  captured no later than the origin.
- **Corroborated origin.** The origin must be a coordinate retained in history,
  and no tick between origin and parent may have written the node or alpha slot
  of the target or of an observed node. A claim that holds at the parent then
  held at the origin too. Because of this, a value that changed and was restored
  is not yet accepted: it falls back to the whole-frontier rule.
- **Within one tick.** Overlapping Actions are still rejected as footprint
  conflicts rather than validated against earlier accepted writes, so no Action
  commits on a state an earlier write in its tick changed.
- **Evidence.** The receipt keeps the origin basis and records the parent in its
  Tick fields, and recovery recomputes the rule from retained history. A failed
  dependency is reported as `BasisChanged`.

The interval write set includes both each retained replay patch's declared
`out_slots` and every record slot written by its operations. This includes
runtime-ingress materialization, whose writes occur outside rule footprints.
Node and edge deletion include their removed attachment slots; portal creation
includes its portal attachment and child root. A missing replay patch or a
warp-instance upsert or deletion makes the interval write set unknown, so the
scoped exception is unavailable and the whole-frontier rule applies. This
corroboration change does not change patch encodings, digests, or roots.

Every other Action follows the whole-frontier rule below.

Today an Action carries its whole `EchoOperationEvaluationBasisV1`: writer head
(worldline and head identities), worldline tick, optional commit global tick,
state root, commit id and application basis. It commits only if that basis equals the
current one, so any commit between evaluation and decision obstructs it with
`BasisChanged`, even when it touched nothing the Action depends on.

Under the decided rule, an Action keeps its **origin basis** (what it was
evaluated against) and may commit onto a different **commit parent** only when
all of the following hold:

- **Complete, evaluator-established dependencies.** The dependency set is what
  the evaluator recorded as the Action's actual reads, preconditions and
  single-key absence checks, enforced against its declared footprint. An author
  cannot earn scoped validity by omitting a read. A path that cannot record and
  enforce its reads, including any topology or collection query, keeps the
  whole-frontier rule.
- **Supported dependency kinds only.** Payload (value) dependencies and
  single-key absence are supported. Revision-sensitive and identity-sensitive
  (incarnation) dependencies are refused at admission until sound tracking
  exists; they are never downgraded to value comparison.
- **Validation at a deterministic position.** Actions in a tick are validated
  in canonical order against the commit parent plus the writes of every earlier
  accepted Action. An Action's own expected values never change.
- **Bound evidence, recomputed on recovery.** The receipt binds the Action
  identity, origin basis, commit parent, validation position, each dependency
  descriptor and comparison mode, and a digest of the compared values. Recovery
  recomputes the validation at that position and refuses a history whose
  result differs; it never trusts the evidence field alone.

An Action is never re-stamped as if evaluated against its commit parent. A
dependency failure names the dependency that changed. The per-tick limit of 64
Actions per head is unchanged. Actions beyond it remain pending for later ticks
and are not obstructed merely because of the limit; each is then validated as
above, and still obstructs if an earlier tick changed one of its dependencies.

`xtask run-edict-operation` owns that local host witness.
`xtask/tests/run_edict_operation.rs` exercises exact external compiler bytes,
malformed and mismatched artifact refusal, input and WAL bounds, and a bounded
fixed-seed stress set. The runner validates the manifest-to-adapter-to-target
configuration chain before Echo independently admits the package. Target
configuration names the key and replacement input fields, node-id derivation,
Action-lane WARP source, authority profile, type profiles, and budget. The
accepted report's Target IR identity must equal the package semantic closure's
Target IR identity, and its result-projection reference must equal the
compiler-owned projection bound into the package. Target configuration is
selected from the adapter implementation whose target intrinsic matches the
package-supported program; an unrelated effect cannot substitute a
configuration with compatible profile and budget values. State validation
checks both the declared node type and the attachment type. The retained report
binds its reading to the input basis and node key, derived worldline and node,
accepted submission, committed Tick, and typed Receipt identities. The
runtime-owned basis selector, configured fields, package meaning, and
verification report all influence the canonical invocation; no application
matcher, executor, footprint callback, or handwritten package participates.

The generic report retains the projection identity, compiler-declared output
type coordinate, exact canonical result bytes, and domain-separated result
identity. The committed Receipt, recovered Action outcome, and recovered WAL
outcome must agree on all four propositions. Corrupting or coherently rebinding
the projection artifact fails independent Echo admission; substituting retained
result bytes fails recovery validation. The machine report exposes the applied
result plus independently comparable fresh-host and WAL-recovered result
records, and fails closed unless all three generic records are exactly equal.
The runner never decodes application-specific fields in production.

The duplicate witness reports the graph-only application-state root and typed
target-value digest immediately before and after the obstructed Action. The
state root commits the entire reachable graph, including attachment values,
while excluding WAL frames, Tick history, Receipts, and commit metadata. The
target digest separately binds the declared attachment type and exact value
bytes. Echo refuses to emit a passing report if either before/after pair
differs.

This is singleton scheduler integration, not a claim that the external
application can compose multiple Actions into one Tick. The kernel's synthetic
multi-Action tests remain separate generic runtime evidence. Product-facing
invocation packaging and the standalone application's own run command remain
downstream integration work.

The operation package owns the public contract and runtime-recognized
invocability. The subordinate program supplies executable meaning only. Echo
cannot install or invoke a naked program digest, and a generated artifact does
not confer caller authority. Provider v1 remains stable while existing
consumers migrate; it is not silently reinterpreted as this new category.
The program bytes bind their interpreter and intrinsic profiles directly.
Direct compatibility commits retain their singleton evidence, while
scheduler-owned Action Ticks use the admitted installed-operation identity for
each receipt-entry rule identity and bind every applied preparation into one
canonical composite consequence. Two packages that reuse identical program
bytes therefore do not collapse into one causal operation identity.
The current slice composes multiple independent, same-basis executable Actions
inside one scheduler-owned Tick. It does not claim cross-category scheduler
composition, Graft-style multi-record mutation, or independent implementation
evidence. Its canonical decoding, identity recomputation, budget and footprint
checks, exact-basis comparison, deterministic repeatability, and WAL recovery
are deterministic self-validation.
For descended targets, the retained footprint and patch inputs include every
portal attachment in the validated root-to-target reachability chain.

The slice currently reuses `TrustedRuntimeHost`, whose module is exposed only
under the joint `native_rule_bootstrap` and `trusted_runtime` feature gate. That
compile-time coupling does not place a callback in the operation program, but
it prevents the maintenance runner from serving as the final product cutover
boundary.
The host/WAL shell must be separated from `native_rule_bootstrap` before Jedit
can delete the legacy feature without also losing executable operations.

## Footprint Honesty

Generated footprints are compile-time claims. Runtime footprint checking in
debug builds is a generator-correctness oracle, not the primary production
authorization mechanism. Release CI must nevertheless execute representative
generated Wesley and Edict packs with `footprint_enforce_release` so a generator
cannot silently ship false declarations.

The `footprint_enforce_release` qualification lane is not wired into CI.
No Wesley or Edict package is currently footprint-release-qualified. The
digest-locked Edict provider package proves publication identity and
reproducibility, not runtime installation or footprint qualification. The Edict
path now has positive provider-native installation and invocation witnesses,
but it still needs a deliberately false-footprint negative oracle before it may
claim release qualification. Wesley still needs both a package emitter and its
own positive installed generated-pack witness.

The scheduler can reserve explicit outgoing-edge range claims, including empty
ranges, as described in the [scheduler contract](../spec/scheduler-warp-core.md).
This claim API does not instrument existing graph reads or edge writers. Range
keys survive patch identity, retained provenance encoding and native Tick
metadata. This does not admit a new authored-effect profile or qualify a
generated package for release.

Under `trusted_runtime`, `edict_effects::EffectRangeReadSession` provides a
host recording primitive for the effect interpreter under construction. The
host selects an immutable frontier, allowed range edge types and a whole-bucket
limit. Setup hashes the real state into a worldline/tick/state-root basis and
indexes the policy outside the per-read budget. This reading coordinate is not
an admitted commit-parent identity: the reachable state root alone does not
bind unreachable by-key data. Admission must also bind the writer head and
commit identity. A forbidden type fails before per-read state lookup, charging
or recording. An allowed read first charges one step and
64 read bytes to resolve the source node, recording presence or absence. If the
source exists, the exact bucket length is checked against the edge limit, then
charged in full at one step and 64 read bytes per edge before inspecting any
edge. Nonmatching types count. A refused charge adds nothing to consumption;
the already-observed source node remains recorded when a later scan refuses.
Successful reads retain the source node and the full source/type range,
including empty and absent ranges; matching edge records are ordered by edge
ID. These are real values observed at the selected basis, not caller claims.

Read value-storage declarations have a fixed 16 MiB ceiling. Every attempted
node or range observation reserves 128 logical bytes; each source-bucket edge
reserves 128 bytes in each of the returned and recorded row buffers, including
nonmatches. Both buffers are provisioned before inspection, and unique edge IDs
are sorted without a temporary heap buffer. Reservations are cumulative across
repeated reads. Storage and I/O charges for each resolution succeed together
or leave both counters unchanged; earlier successful resolutions remain
recorded when a later one refuses. A host that cannot provide admitted buffer
capacity reports `HostAllocationFailed`, not `BudgetExceeded`. That host fault
must stop evaluation and must never become an Action obstruction or retained
decision.

`EffectEvaluationFrame` connects that view to the real compiler-package pure
interpreter through one cumulative step and allocation meter. It consumes an
unused session and intersects the interpreter's ceilings with the session's
limits. Current frame declarations are bounded by 1,048,576 steps, 64 MiB of
read bytes, 16 MiB of write bytes and 16 MiB of value storage. These host-frame
bounds do not establish an admitted operation-profile contract. An already-used session cannot reenter as unspent capacity. Each read
must fit both the view's I/O limits and the shared computation/storage budget;
failed read reservations change neither meter. Each pure component retains its
own declared ceiling relative to its starting usage, while the absolute frame
ceiling continues to bind all components together. Component limits are restored
between calls without refunding work or storage. Any identity, input,
computation, read or resource failure ends the frame; later calls do no work.
The shared interpreter uses atomic failed charges. An admitted concatenation
buffer that cannot be provisioned is a host fault, never budget exhaustion.
The existing standalone pure/read APIs retain their prior accounting behavior.

`TrustedRuntimeHost::effect_evaluation_frame_v1` can now construct that frame
against an expected full runtime parent. The host resolves the registered writer,
worldline tick, committing global tick, state root and commit identity through
the existing scheduler basis resolver, compares all runtime fields, and borrows
its own frontier. Equal reading coordinates from a different commit history are
refused before constructing a frame. The immutable host borrow prevents a
scheduler tick during the frame's lifetime. The typed application basis is
carried as a proposition; this constructor does not validate application input
or grant invocation authority. A frame constructed by this host method exposes
its full `evaluation_basis`; low-level `EffectEvaluationFrame::new` frames expose
`None` and cannot mint a runtime-bound coordinate from caller-supplied storage.
This is private host composition, not an installed or admitted effect profile.
The state-root/by-key caveat of the low-level reading coordinate and the
package-admission and recovery obligations remain as described above and below.
The optional typed mode below constrains runtime-bound reads separately.

A runtime-bound frame can also run an unchanged
`compiler-produced-bounded-read/v1` package through `evaluate_read`. It uses the
same decoded instructions and expression interpreter as standalone
`edict_read::evaluate`, but its access adapter resolves the frame's own frontier
and records dependencies. The trusted caller supplies a sorted, unique bounded
node aperture. A key outside it is refused before graph observation. An unbound
caller-frontier frame cannot enter this recorded interpreter path.

Each source-node resolution charges one step, 64 read bytes and 128 logical
storage bytes before lookup, and records presence or absence. A missing node
stops that read without resolving its attachment. For a present owner, resolving
its alpha attachment charges one step, 64 read bytes plus any atom payload
length, and 128 logical storage bytes plus that payload length before retention
or value delivery. The interpreter retains its existing byte-copy work and
value-storage charges. Source and attachment reservations are individually
atomic across the read and shared meters. A later failure retains the resolved
prefix, including negative observations. Wrong types, oversized atoms, missing
attachments and authored guards keep their existing obstruction coordinates;
the actual observations remain distinct from the caller's expected values.

The frame intersects package/input/output ceilings with each requested read
component, preserves its declared local limits, and keeps work/storage consumed
by earlier reads and computations. Every refusal or host fault terminates the
frame. Failed observation reservations add no read/storage charge, and host
allocation failure while provisioning precharged atom bytes remains a host
fault rather than an authored obstruction. Standalone bounded-read accounting
and outputs are unchanged. This legacy entry point uses an untyped trusted-host
node aperture. It is refused before work or observation after a typed law is
bound to the frame. It grants no application mutation or admission authority.

An unused runtime-bound frame can bind an `EffectFootprintLaw` through
`with_footprint_law`. The decoder requires canonical CBOR with exactly the nine
footprint-law fields: six type-id sets and `max_reads`, `max_writes`, and
`max_range_edges`. Set arrays contain distinct, strictly ascending 32-byte IDs;
unsorted or repeated entries are refused rather than normalized. Zero counts
are allowed. Read, write and whole-bucket ceilings are 4096, 256 and 65536.
Decoding is bounded to 16 MiB and occurs during trusted-host preparation. The
write permissions are validated as data but grant no write API. Binding refuses
an unbound frame, previous work, a prior failure, or replacement of an existing
law. These are host-selected permissions, not an authenticated package policy
or admission token.

Typed mode provides `read_typed_node`, `read_typed_range`, and
`evaluate_typed_read`. Present nodes and atoms are authorized by their actual
stored types. `TypedNodeRead::absent_node_type` authorizes node absence only;
absent atoms use the requested atom type. A present, authorized atom with a
different requested type still yields the existing authored type-mismatch
obstruction. Node and atom permissions are checked before retaining or
returning their values. Raw slot readings retain authorized resolved values;
`authorized_reads` additionally retains distinct slot/type pairs, so absence
under two named types does not collapse into one authorization. `max_reads`
bounds those pairs after deduplication. Permission roles are checked on every
resolution even when a pair already exists: a portal-owner permission does not
permit an ordinary node read. Range pairs carry a zero read type, since their
edge type is already in the range key. Range edge types must pass both the
host's policy and the law before any state access. Source nodes use their
actual or absent type, and the law's bucket ceiling intersects the host limit.
Legacy untyped range and compiled-read entry points cannot bypass a bound law.

Before a typed read resolves a descended-warp target, it follows bounded
instance metadata back to the root or a previously validated ancestor. It then
resolves and authorizes alpha portals root-first. Each portal contributes its
actual owner-node and descent-attachment observations, both authorized by
`portal_owner_types`. Edge-owned or non-alpha portals are refused before an
owner record is read. Unknown, cyclic or disconnected chains report
`UnreachableWarp`; no target value is resolved. A validated chain is reusable
within the frame's immutable parent. This cache never grants a different
permission role. The low-level untyped session retains its existing by-key
behavior.

Typed resolutions keep the existing atomic observation and payload charges.
Each new authorization pair reserves 64 additional logical storage bytes. Each
uncached reverse-metadata hop costs one step, 64 read bytes and 256 logical
storage bytes before extending its bounded chain/cycle indexes; actual portal
owner and attachment resolutions separately cost one step, 64 read bytes and
128 observation bytes each. These are private-frame charges, not a shipped
operation-profile recipe or a physical allocator limit. Failed charges change
neither meter. Any law, reachability, resource or authored failure ends the
frame. Accepted authorization pairs can precede a later payload-budget failure;
they are host-private authorization evidence, not the persisted canonical
`ReadRow` image. The executable-operation pipeline tests exercise real bound
frontiers, compiled reads, nested alpha portals, beta refusal, distinct named
absence, role separation, whole-bucket limits and atomic failure prefixes.

`EffectEvaluationFrame::into_read_set_image(max_bytes)` consumes a typed,
runtime-bound frame into an owned, eight-byte-aligned `EffectReadSetImage`.
Successful and failed frames can export their completed observation prefix.
Permission reservations without a completed raw value resolution are omitted;
a failed atom payload charge cannot turn into a false absence row. Unbound or
untyped frames report `MissingTypedParent`. The owned export preserves the
actual full runtime parent, cumulative work/I/O and logical allocation usage
separately from the image bytes. Consuming the frame prevents further evaluation
against an already-exported read set.

The image is the read-set digest preimage after its domain: a little-endian
`u64` row count, the 192-byte `EffectReadRow`s, a little-endian `u64` pair count,
and the 64-byte `EffectEdgePairRow`s. There is no new retained-history record
magic. A read row contains a 104-byte `EffectSlotRow`, outcome and zero padding,
a 32-byte authorizing type, a 32-byte subject digest and a WSC range descriptor.
All rows are `repr(C, align(8))`, `Pod` and `Zeroable`, with explicit padding.
The slot classes are node `0`, attachment `1`, edge `2`, range `3`. Their wire
order is explicit and differs from native `SlotId` ordering. Slots sort by
class, warp, local ID, range edge type, attachment owner and plane; read rows
then sort by authorizing type. A present slot has one actual-type row. Only
absent slots can repeat under distinct ascending named types.

Node subjects hash `echo.bridge.node-value.v1\0` followed by the node type.
Atom subjects hash `echo.bridge.attachment-value.v1\0`, tag `0`, atom type,
little-endian byte length and bytes. Portal subjects use that attachment domain,
tag `1` and the child warp ID; their read type is the owner's node type. Edge
subjects hash `echo.bridge.edge-value.v1\0`, edge type, source and target local
IDs. Absent subjects are zero. Range subjects and read types are zero; the range
slot already names its edge type. Matching edge/target pairs are packed in
range-row order, sorted and unique by edge local ID within each run. Separate
source-node rows distinguish an absent source from a present empty bucket.
The read-set digest hashes `echo.bridge.read-set.v1\0` and the exact image.
The parent and accounting metadata are not part of this read-set digest; an
admitted proposal/decision must separately bind them.

`EffectReadSetView::parse` borrows rows and pairs directly from an aligned input
buffer. Its structural validator allocates no collections and checks exact
framing, checked size arithmetic, tag legality, zero padding/unused fields,
canonical order, duplicate rules, pair bounds, strict edge order and contiguous
complete pair-section coverage. A suffix, an orphan pair or overlapping range
is invalid. The fixed ceiling is 4096 read rows and 65536 pairs per range, with
an image no larger than 16 MiB. The total pair section can exceed 65536 when
individual ranges and the image limit permit it. This view proves canonical
structure only; it does not authenticate a law, parent, dependency completeness
or historical Action decision.

Export is bounded host retention work, outside the consumed evaluation budget.
The caller's output limit intersects the fixed 16 MiB ceiling; the temporary
sorted descriptor table is bounded by the authorized-read ceiling. Export uses
only completed recorded values and does not resample the frontier. Provisioning
failure reports `HostAllocationFailed`, while an insufficient output allowance
reports `ExportLimitExceeded`; neither is an authored Action obstruction.
These buffers are separate from the interpreter's logical reservations. This
codec does not implement the single physical arena, receipt/WAL retention,
proposal admission or fresh-engine effect recovery. Independent wire fixtures
exercise malformed inputs, maximum counts, alignment and field encoding; real
runtime witnesses exercise exact node bytes/digests, atoms, portals, absence,
range packing, failed prefixes and preserved parent/resource metadata.
The codec integration target declares `trusted_runtime` in Cargo's
`required-features`; CI explicitly runs and lints it in the trusted-runtime
lane. The feature-gated-test scheduling guard rejects missing declarations or
missing feature-qualified CI execution.

This is trusted-host composition of pinned pure packages and recorded reads.
It provides no mutation or admission path and is not a compiler-authored
effect-program format. Package/input decoding remains bounded host preparation.
Logical value-storage reservations bound the row buffers and observation count;
policy preparation, indexes and allocator overhead remain host storage. The
physical interpreter arena, package-authenticated footprint law, receipt/WAL
integration of typed read images, authored outgoing-range/effect instructions, derived effects and
installed-profile validation remain required before a new authored-effect profile can ship.
Existing package profiles and graph accessors are unchanged.

Conflicts remain explicit receipt rejections. A footprint failure must not
trigger hidden retries or widen access.

## Required Guards

- Default builds do not expose raw `RewriteRule` construction or public raw
  registration.
- Product and adapter crates do not enable `native_rule_bootstrap`.
- Generators emit artifacts, descriptors, and opaque proposals without
  registering themselves. Trusted hosts separately admit proposal claims,
  corroborate exact package evidence, and register package-qualified generated
  material through the appropriate provider-native or Wesley compatibility
  record—not an app-specific engine escape hatch.
- Newly authored executable operations resolve installation and invocation from
  an admitted `ExecutableOperationPackage`; a naked
  `EchoOperationProgram` cannot install or invoke.
- Newly authored executable operations expose no application matcher, executor,
  footprint callback, or prebuilt mutation plan. Provider-v1 callback bindings
  remain explicitly labeled compatibility infrastructure.
- Registry/package identity, operation identity, codec/schema compatibility,
  and footprint metadata are verified before the engine mutates registration
  state.
- Provider invocation requires the exact EINT intent kind, an installed
  provider operation, and receipt evidence from the exact bound scheduler rule.
- Runtime patches and receipts retain the generated package or rule-pack
  identity needed to explain execution.


### Executable graph-schema preparation

`EffectGraphSchema::from_canonical_bytes` decodes the graph contract required by
the current authored-effect program contract. The root has exactly `node_types` and
`edge_types`. A node-type entry has exactly `addressing` (`0` content or `1`
mutable) and `atom_types`; an edge-type entry contains source/target node-type
pairs. Every identifier is exactly 32 bytes. Set arrays are strictly ascending
and distinct, including lexicographic ordering by both endpoint types. Invalid
order or duplicates refuse rather than being normalized.

The prepared value answers node-addressing, node/atom compatibility and edge
endpoint-type membership queries. Empty relations deny membership. This is
schema preparation, not graph-record resolution, identity-law enforcement,
package authentication or Action admission. It creates no read or mutation
capability. Input is bounded to 16 MiB and canonical decoding retains its
combined node/depth ceilings; these host preparation allocations are outside
the evaluation meter and do not implement its physical arena.

### Current operation publication preparation

`ExecutableOperationPackage::prepare` checks a trusted host's expected current
package identity against exact canonical publication bytes. There is one
`ExecutableOperationPackage`, one `EchoOperationSemanticClosure`, and one
`EchoOperationProgram` definition and codec. The program kind selects
compare-and-set, create-if-absent, or the authored-effect payload. The former
V2 package definition and decoder have been removed without aliases, wrappers,
or schema-version dispatch.

The current package and program schema strings are `echo.operation-package/v1`
and `echo.operation-program/v1`. Their identities use the corresponding
length-prefixed `echo:operation-package:v1\0` and
`echo:operation-program:v1\0` domains. These are labels for the sole current
formats; the former V2 publication/program schemas are refused even when their
bytes have a matching current-domain pin.

The shared package envelope binds the operation, obstruction, authority,
semantic closure, exact nested program, budget, profile identities and footprint
contract. Fixed programs require a real paired lawpack coordinate and identity.
Authored effects forbid both lawpack fields and the application-result
projection; their closure has exactly the six upstream source/meaning/Core/
Target IR/schema fields. No sentinel lawpack is introduced. The closure and
effect program must bind the same Target IR identity.

The authored-effect program payload has exactly thirteen fields. Its
`edict-effect-program/v1` kind requires the declared effect interpreter and
intrinsic profiles, ContentAddressV1, exact executable graph schema, exact
footprint law, compiled bytes, output type and resource ceilings. Its footprint
identity is BLAKE3 over `echo:operation-footprint-law:v1\0` followed directly by
the canonical law bytes. The package's effect target, input, result, obstruction,
interpretation and basis profiles are checked by the same package validator
that validates the fixed program kinds.

Input and result declarations permit zero and are capped at 65,536 bytes.
Allocation permits zero and is capped at 16 MiB. The output coordinate is
nonempty UTF-8 of at most 1,024 bytes; other coordinates retain the current
nonempty-text rule. Effect package steps must be positive and byte budgets may
be zero. Preparation is outside evaluation metering and does not provision the
physical evaluation arena.

This is byte binding and contract preparation. Compiler bytes and upstream
identities do not infer verifier approval or validate the compiled instruction
language. The invocation definitions described below still require consolidation under the one-current-
version policy. Preparation does not grant invocation or mutation authority.

### V2 invocation input and basis preparation

`EffectInvocationV2::prepare` checks an exact canonical V2 invocation against a
prepared current publication. Only its seven specified fields are accepted. The
package identity and operation coordinate must match the publication. The
application input must itself be canonical CBOR and fit that program's declared
input ceiling, including the input's CBOR framing bytes. The delegated budget
requires at least one step and cannot exceed any package budget axis.

The invocation retains the existing V1 evaluation-basis map. Its application
basis schema is the effect-basis profile, and its application basis value is
BLAKE3 over `echo:operation-edict-effect-basis:v1\0`, the input byte length as a
little-endian u64, and the exact input bytes. An input change requires a new
basis digest and changes the versioned V2 invocation identity. The decoder
reuses V1's basis parser without changing its encoding or identity recipe.

This is static preparation. It retains a claimed grant identity and parent;
it does not verify a grant, establish installation, resolve the current
frontier or admit an Action. Runtime-owned authority checks and evaluation at
the named parent remain required before effects can be accepted.

### Runtime-owner package policy admission

`TrustedRuntimeHost::admit_echo_operation_package` is the sole package-admission
crossing. It consumes the current `EchoOperationAdmissionPolicy` and exact
canonical publication bytes for any supported program kind, returning one
opaque `AdmittedExecutableOperationPackage`. The former version-specific owner
methods, effect-admission module and effect token definition were removed
without aliases or wrappers. Retained recovery reruns the same validator.

The independently selected policy pins package identity, operation coordinate,
authority profile and maximum budget. The common validator checks the exact
pin, canonical artifact and profile contracts, operation/authority agreement,
and every budget axis. A matching pin does not replace structural validation
or independent authority policy. Both fixed and authored-effect programs cross
these same checks and produce the same current evidence type.

The token retains the prepared package, exact publication bytes, checked
package/program identities, policy and policy identity. Its admission identity
uses `echo:operation-package-admission:v1\0` followed by the raw package and
policy identities. There is no alternative effect-admission identity recipe.
Policy identity uses the existing current
`echo:operation-admission-policy:v1\0` recipe. Different independent policies
therefore retain distinct admission evidence for otherwise identical bytes.
Installation consumes the token's checked program identity; recovery rebuilds
and checks this evidence rather than trusting retained hashes alone.

This method is exposed only on the runtime-owner host, under the existing
`native_rule_bootstrap` and `trusted_runtime` boundary. The restricted
application handle cannot admit a package. The token has no public constructor
and is neither an installation nor an invocation grant. Installation uses one evidence type and catalog for every program kind.
Invocation admission and compiled effect execution remain unfinished. Package
admission does not infer compiler/verifier approval from publication identities.

### Runtime-owned installation and retained catalog recovery

`TrustedRuntimeHost::install_admitted_echo_operation_package` consumes the
opaque common package-admission token and installs one `InstalledEchoOperation`
in the engine's sole package catalog and operation-coordinate index. Fixed and
authored-effect program kinds use the same owner API, evidence type, preflight,
retained codec, recovery collection and restoration path. The former effect
installation module/type, second catalog/index, version-dispatch decoder and
version-specific methods were removed without aliases or wrappers. Application
handles cannot call the owner method.

The installed object retains the exact admitted package/program/policy evidence.
A lawpack identity is `Some` for fixed programs and `None` for authored effects;
absence is not a sentinel hash. Installation grants no invocation authority and
does not validate the compiled effect instruction language. Its read-only
`retained_bytes` and `from_retained_bytes` methods reconstruct evidence without
inserting registry entries.

Exact repeats are idempotent. A changed policy for the same package or a changed
package at an occupied operation coordinate refuses before any WAL append,
regardless of program kind. Batch recovery preflights the same single catalog
before publication. The installation identity uses
`echo:operation-installation:v1\0`, then raw package identity, package-admission
identity, length-prefixed operation coordinate, program identity and
length-prefixed exact package bytes, in that order.

The sole retained installation record has exactly seven fields with schema
`echo.operation-installation/v1`: independent policy, policy identity, exact
package bytes and identity, package-admission identity and installation identity.
Recovery reruns the common package-policy validator, recomputes the identities
and reproduces exact record bytes. Unsupported installation schemas, including
the former V2 schema, refuse. Recomputed hashes cannot make a false independent
policy acceptable.

With a runtime WAL configured, installation commits before the registry entry
becomes visible. Failed append/commit paths publish no entry. Uncertain
filesystem errors refresh the writer and compare the exact recovered
installation, including policy and program evidence, before reporting success.
A commit-marker sync failure leaves the live adapter closed and publishes no
entry; a fresh store reopen may establish and recover the committed history.
Without a WAL, installation is volatile; an empty WAL cannot attest an earlier
volatile installation. These checks apply to every program kind.

The current receipt type has nullable lawpack evidence. Its canonical field
is either a 32-byte identity or null; its identity hash includes a presence byte
and, when present, the raw identity. Absence therefore differs from a present
zero hash. Receipt corroboration checks the value against the exact installed
program evidence; a structurally valid absent claim cannot replace a fixed
program's real lawpack. This updates the one current receipt recipe rather than
retaining an older decoder or hash path.

The existing installation transaction and catalog-frontier recipes bind the
exact retained bytes. Recovery certificates use one current operation-index
recipe, under `echo:trusted-runtime-wal:executable-operation-index:v3\0`, for
every catalog and outcome shape. It hashes the preceding index root, then three
count-prefixed sections: installations, execution receipts, and Action outcomes.
Each record is length-prefixed. Installations are ordered by raw package identity
and then retained bytes; receipts and Action outcomes retain their causal order.
Action-outcome records bind the submission and ingress identities as well as the
typed outcome. Empty sections contribute their zero counts, including when all
three sections are empty. There is no older index recipe, empty-index bypass,
or recipe switch based on the presence of effect installations or Action outcomes.
The historical index recipes were removed under the one-current-version policy.

A fresh filesystem-backed host restores all supported program kinds through
the sole installation collection and rechecks coordinate conflicts. Invocation
definitions still require consolidation under the one-current-version policy.
This is installation recovery, not recovery of
an authored-effect decision. Invocation grant admission, compiled execution,
physical allocation, derived effects and Action/Tick/receipt recovery remain
unfinished.
