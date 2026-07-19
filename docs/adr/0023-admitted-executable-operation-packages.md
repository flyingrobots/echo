<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0023: Admitted Executable Operation Packages

- **Status:** Accepted
- **Date:** 2026-07-18
- **Partially supersedes:** ADR 0015 for newly authored executable operations

## Context

Echo's provider-v1 corridor witnesses an exact package, operation, Target IR,
and scheduler-rule identity, but it obtains application execution semantics
from host-supplied matcher, executor, and footprint function pointers. The
source explicitly says that matching callback claims do not prove callback
semantics.

Edict's current Echo Target IR is a target review artifact. It names effects,
target intrinsics, requirements, budgets, obstruction mappings, and a result,
but it is not an executable graph-transformation program. Echo's current rule
type likewise stores native callbacks; its pattern metadata is not an
interpreted DPO rule.

The first real convergence witness is Jedit's `ReplaceRange`. Its current
handwritten planner contains substantial application meaning: exact-basis
validation, fixed-width byte coordinates, range and UTF-8 checks, no-op
refusal, persistent-rope splitting and balancing, content-addressed fact
construction, version advancement, footprint derivation, and result facts.
Moving that planner into Echo or hiding it behind a native target intrinsic
would preserve the callback defect under another name.

The new corridor is therefore not provider v2. Provider v1 remains stable
callback-shaped compatibility infrastructure while consumers migrate. The new
semantic category is an executable operation package whose admitted program is
interpreted by Echo without an application matcher, executor, footprint
callback, prebuilt mutation plan, or native operation implementation.

## Decision

### Executable operation category

Echo will add an operation-oriented corridor with these provisional names for
the first version:

- `ExecutableOperationPackageV1`: exact publication material and provenance;
- `EchoOperationProgramV1`: the target-relative executable program artifact;
- `AdmittedExecutableOperationPackageV1`: package-admission evidence;
- `InstalledEchoOperationV1`: Echo-owned installed executable meaning;
- `PreparedEchoOperationV1`: one basis-bound private evaluation result;
- `EchoOperationExecutionEvidenceV1`: receipt and recovery evidence.

These are new types and propositions. They are not aliases for provider-v1
hooks or records. Lawful package-byte corroboration, registry conflict checks,
canonical codecs, scheduler integration, WAL framing, and recovery machinery
may be reused when their propositions remain exact.

`EchoOperationProgramV1` is deliberately target-relative. It does not claim to
be the universal WARP wire noun or a runtime-neutral graph-program standard.
The name may be superseded after more than one real operation and runtime can
demonstrate a common portable boundary.

The alternatives considered for the executable artifact name were:

- `ContainedWarpOperationV1`: rejected for the first version because it implies
  stronger recursive-WARP realization than the implementation demonstrates.
  Version 1 has no durable child lane, child-local tick history, wormhole, or
  recursively schedulable child runtime.
- `BoundedGraphOperationV1`: rejected for the first version because it reads as
  a runtime-neutral or universal graph-operation contract before that claim has
  cross-runtime evidence.
- `EchoOperationProgramV1`: selected because it states the current truth: this
  is a bounded program for Echo's target profile.

### Program Substitution Boundary Law

An `EchoOperationProgramV1` digest is subordinate executable evidence. It is
not an operation coordinate, an installed operation, an invocation capability,
an authority token, or permission to invoke anything. Possessing, publishing,
or corroborating program bytes cannot make them independently installable or
invocable.

The binding direction is explicit:

```text
Edict intent / admitted executable-operation package
→ binds the canonical semantic closure
→ binds the exact Jedit lawpack coordinate and digest
→ binds the exact EchoOperationProgramV1 bytes and digest
```

The admitted Edict operation supplies the public operation coordinate,
invocation and result schemas, basis contract, budget and footprint contract,
authority requirements, result and obstruction interpretation, and the
runtime-recognized eligibility to invoke the operation. Echo still admits each
invocation against those requirements; package bytes do not authorize a
caller. The program supplies only the executable meaning used to evaluate an
already admitted invocation.

Echo must therefore begin installation and invocation lookup from an admitted
operation-package identity, then follow its closed semantic bindings to the
program. It must not admit, install, or invoke a naked program digest. Any
operation or schema references carried inside program bytes are consistency
claims checked against the admitted package, not declarations that mint a
public contract. Reusing identical program bytes in another lawful package
does not merge operation identities, invocability, or authority. Substituting
different program bytes changes the semantic closure and package identity.

### First-version execution obligations

Every `ExecutableOperationPackageV1` must canonically bind:

- one public operation coordinate and exact invocation, output, and
  obstruction schemas;
- the Edict source, canonical meaning, Core, and target identities that define
  the admitted semantic closure;
- the exact Jedit-owned canonical text-schema declaration coordinate and
  digest;
- the exact lawpack resource coordinate and digest;
- the exact `EchoOperationProgramV1` bytes and digest;
- an explicit parent-basis contract;
- delegated budget and declared footprint contracts;
- authority requirements and invocation-admission requirements;
- result and obstruction interpretation;
- one evaluator ABI and intrinsic-profile identity.

Every bound `EchoOperationProgramV1` must canonically bind its executable
meaning:

- one invocation-to-graph binding;
- a typed graph and attachment schema;
- a closed declarative rewrite/evaluation program;
- deterministic rule and match selection;
- only versioned, digest-locked, deterministic low-level intrinsics;
- static resource requirements or ceilings checked against the delegated
  budget;
- executable footprint derivation checked against the declared footprint
  contract;
- typed result and obstruction construction checked against the public
  operation schemas and interpretation;
- one evaluator ABI and intrinsic-profile identity;
- atomic visibility as one parent patch or one obstruction.

The evaluator may use private deterministic working state and several bounded
internal steps. That state is not parent history. A successful evaluation
exposes one normal Echo patch; an obstructed evaluation exposes no parent
mutation. Version 1 retains an execution or trace identity sufficient to bind
later evidence without claiming that a durable child worldline already exists.

The program may use generic primitives for typed graph access, canonical scalar
operations, checked `u64` arithmetic, bounded byte manipulation, UTF-8
validation, canonical hashing, content-address derivation, and graph-delta
construction. Echo must not provide an intrinsic whose native implementation
is Jedit `ReplaceRange`, persistent-rope replacement, or another application
semantic operation.

### Prepared Operation Basis Law

A prepared operation may commit only against the exact parent basis on which
it was evaluated.

The first version names one canonical basis value,
`EchoOperationEvaluationBasisV1`. It is not a choice among a tick, commit, root,
or application coordinate. Every encoded value contains these fields in this
order:

1. `writer_head.worldline_id`: the canonical 32 bytes of `WorldlineId`;
2. `writer_head.head_id`: the canonical 32 bytes of `HeadId`;
3. `worldline_tick`: one little-endian `u64`;
4. `commit_global_tick`: a required option encoded as tag `0` for the empty
   `U0` frontier or tag `1` followed by one little-endian `u64`;
5. `state_root`: the exact 32-byte canonical graph-state root;
6. `commit_id`: the exact 32-byte canonical frontier/commit hash;
7. `application_basis_schema_digest`: the exact 32-byte digest of the basis
   schema bound by the admitted operation package;
8. `application_basis_value_digest`: the exact 32-byte digest of the canonical
   basis value carried in the package-declared invocation field.

Its canonical byte encoding is the ASCII domain
`echo:operation-evaluation-basis:v1\0` followed by those fields without padding.
Its identity is the BLAKE3 digest of those bytes. The option tag is mandatory;
all other fields are mandatory. The admitted operation's basis contract names
exactly one invocation field and codec. Echo computes the application-basis
value digest from that field's canonical bytes and resolves the value to the
Jedit buffer head present at the named runtime frontier. A missing, foreign,
stale, or differently encoded application basis is a typed obstruction; Echo
does not choose or infer another basis.

`PreparedEchoOperationV1` and its eventual execution evidence must bind:

- the complete `EchoOperationEvaluationBasisV1` value and identity;
- the exact submission/invocation identity, canonical invocation digest,
  caller-authority evidence identity, and Echo-owned invocation-admission
  evidence identity;
- the admitted operation and package identities, package-admission evidence
  identity, Echo-owned installed-operation identity, and subordinate program
  identity;
- the delegated budget and consumed budget;
- the declared footprint identity;
- the actual read/write footprint derived during evaluation;
- the private execution or trace identity;
- exactly one closed `PreparedEchoOperationOutcomeV1` variant:
    - `Committable`, carrying a patch digest and typed output identity; or
    - `Obstructed`, carrying a typed obstruction identity and, by its canonical
      variant encoding, explicit evidence that no parent patch exists.

An `Obstructed` outcome never enters parent commit or scheduler composition and
never carries a patch digest. It may produce retained typed execution evidence,
but it does not claim a committed consequence.

Before any `Committable` outcome can commit, Echo must establish all of the
following:

```text
current EchoOperationEvaluationBasisV1 bytes == evaluated basis bytes
current installed admitted-operation identity == evaluated operation identity
current installed package identity == evaluated package identity
current installed program identity == evaluated program identity
admitted invocation identity and admission evidence == evaluated identities
canonical invocation identity == evaluated invocation identity
consumed budget <= delegated budget and evaluation completed within that budget
actual footprint is permitted by the declared footprint contract
```

An attempted evaluation that cannot complete without exceeding its delegated
budget yields a typed budget obstruction and no committable patch. A successful
evaluation may consume exactly its delegated budget if it completes without an
overrun.

For a singleton commit, Echo must additionally establish:

```text
committed patch identity == evaluated Committable patch identity
```

An existing scheduler composition rule may combine independent committable
preparations evaluated from the same basis. The committed receipt must then
bind the exact scheduler-rule identity, a canonical ordered list of member
preparation and patch identities, and the resulting composite `TickPatch`
identity. The scheduler law must verify that every member has identical
`EchoOperationEvaluationBasisV1` bytes and that the composite patch is the
canonical result of that exact ordered membership. A singleton commit uses the
same evidence shape with a one-member list.

If the parent basis changed, the prepared operation is ineligible to commit and
must yield a typed basis-changed posture without applying any patch. Echo must
not silently rebase, retarget, revalidate, or transport the preparation to the
new basis. A new evaluation is a new witnessed attempt. Scheduler composition
from one unchanged common basis is not cross-basis revalidation.

`EchoOperationExecutionEvidenceV1` has a separate closed terminal sum:

- `Committed` binds the composite `TickPatch` digest, typed output identity,
  resulting frontier identity, and scheduler composition evidence; or
- `NotCommitted` binds the evaluation obstruction or commit-ineligibility
  identity, a canonical terminal-outcome digest, and canonical no-parent-patch
  evidence.

`NotCommitted` never carries a committed patch digest. A preparation that was
`Committable` but lost exact-basis eligibility is represented as
`NotCommitted`; its uncommitted prepared patch remains attributable but is not
misreported as a parent consequence.

When an admitted package declares no application-specific caller-authorization
requirement, the caller-authority evidence field binds the versioned canonical
`None` identity selected by that package profile. It is never omitted or
silently replaced by package or program identity. This requirement binds the
admission posture without adding broad authorization policy to this decision.

### Jedit rope-law closure

For the first `ReplaceRange` vertical, Jedit will use a digest-locked canonical
declarative executable semantic resource imported through a Jedit-owned Edict
lawpack. This decision does not require Edict source to express the complete
recursive persistent-rope algorithm directly.

Before the real package can be generated or admitted, Jedit must publish one
versioned canonical text-schema declaration. That Jedit-owned declaration must
bind:

- the exact fact, edge, and attachment proposition set and schema digest;
- the canonical codec and content-identity law for each proposition;
- whether the TypeScript-only structural-maintenance and checkpoint-anchor
  propositions are authoritative retained facts, derived evidence, or excluded
  from the first operation surface;
- the compatibility or migration posture for the current native JSON fact
  model; and
- the exact rope-law resource coordinate and digest that consumes the selected
  schema.

The executable-operation package must bind that declaration's coordinate and
digest. Its public schemas, lawpack, and subordinate program must close exactly
over the declared propositions and codecs. Absence or disagreement produces a
typed schema-closure refusal before package admission or installation. Edict
and Echo validate the selected declaration; neither may choose between Jedit's
current TypeScript and native models.

The authoritative binding chain is:

```text
Jedit-owned ReplaceRange.edict
→ admitted executable-operation package and public operation contract
→ exact Jedit canonical text-schema declaration
→ canonical semantic closure and exact fact/codec/identity law
→ exact Jedit lawpack coordinate and digest
→ exact canonical declarative EchoOperationProgramV1 bytes and digest
```

The Edict intent remains the application-owned operation surface. Its canonical
meaning binds the lawpack resource and program digest. Changing the executable
resource changes the admitted semantic closure and package identity.
The program digest alone does not confer the operation coordinate,
invocability, caller authority, result interpretation, or permission to enter
Echo admission.

The resource is acceptable only when it contains the executable meaning. It
must contain the declarative rewrite/control law needed to derive the rope
consequence from the invocation and parent graph. It may not contain or resolve
to:

- a Rust function pointer;
- a callback coordinate;
- a host trait implementation;
- a generated `MutationPlan` supplied by Jedit;
- a native `replace-range` intrinsic;
- an assertion that ambient code implements the named operation.

Jedit owns the resource and its exact fact/codec law. Edict binds it into
canonical source and Core meaning and routes it through the selected target.
The Echo target lowerer emits the target artifact and package bindings. Echo
validates, admits, installs, evaluates, schedules, commits, receipts, retains,
and recovers the exact program without learning Jedit semantics from ambient
host code.

The current handwritten Jedit planner may remain temporarily as test-only
differential evidence. It must be unreachable from production execution after
cutover.

## Alternatives for Expressing the Rope Law

### Expand Edict source to express the full operation directly

This route would add enough source and Core language to author persistent-rope
replacement directly: fixed-width values, bytes, variants, optionals,
conditionals, checked arithmetic, recursion or bounded iteration, typed graph
access, content-addressed construction, private work state, result projection,
and complete obstruction control.

This is the preferable long-term authoring experience if repeated operations
demonstrate that those constructs belong in Edict. It is rejected for the first
vertical because the current compiler's initial lowerable subset is much
smaller. Implementing the entire language surface before any hook-free
operation runs would turn the first witness into a broad language and runtime design
effort and would make `ReplaceRange` define unproven universal syntax.

### Bind a canonical declarative executable resource from a Jedit lawpack

This route adds only the Edict prerequisites needed to type the real operation,
name an explicit basis, import and bind the lawpack resource, preserve its
identity through Core and target lowering, and generate canonical client and
package artifacts. The persistent-rope execution law lives in declarative
program bytes rather than native code.

This route is selected. It is the smallest route that closes the execution
proof chain without broad Edict expansion. It does not establish a permanent
rule that complex Edict operations must be authored as external resources.
Later Edict syntax may compile to the same target program boundary.

## Verification Evidence Grades

The word "independent" is reserved for an implementation that does not merely
enter the same logic through another crate or call path. Evidence claims use
the following grades.

| Check                                                                                                         | Evidence grade                                                                                                        | First-version claim                                                                                                                                      |
| ------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Canonical decoding, schema closure, digest recomputation, supported instruction set, and static budget bounds | Deterministic self-validation                                                                                         | The exact bytes are internally well-formed under the installed ABI.                                                                                      |
| Installed package/program/input identity and exact-basis equality before commit                               | Deterministic self-validation                                                                                         | Echo committed only the admitted preparation against its evaluated basis.                                                                                |
| Submission, caller-authority, and invocation-admission evidence identities                                    | Deterministic self-validation                                                                                         | Preparation and retained execution evidence attribute evaluation to the exact Echo-admitted invocation.                                                  |
| Consumed budget against the delegated budget                                                                  | Deterministic self-validation                                                                                         | An evaluation that cannot complete within budget obstructs without a committable patch.                                                                  |
| Actual footprint against declared footprint law or ceiling                                                    | Deterministic self-validation                                                                                         | The evaluated support stayed within the admitted footprint contract.                                                                                     |
| Scheduler rule, ordered preparation membership, and composite patch                                           | Deterministic self-validation                                                                                         | The committed tick is attributable to the exact composition decision over one common basis.                                                              |
| Core to Target IR relation                                                                                    | Structurally separate verifier path                                                                                   | A separately invoked verifier path recomputed or checked the declared relation; separation alone is not implementation independence.                     |
| Admitted operation package plus Target IR and lawpack resource to `EchoOperationProgramV1` binding            | Structurally separate verifier path                                                                                   | A separately invoked target verifier checked exact package/program/resource/schema/profile correspondence without making the program an authority token. |
| Generated codec and package golden bytes                                                                      | Independently implemented conformance evidence only when the comparison implementation shares no encoder/lowerer path | The checked finite vectors agree; this is not a proof over all values.                                                                                   |
| Program result and patch versus the existing handwritten Jedit planner over a frozen differential corpus      | Independently implemented conformance evidence for that finite corpus                                                 | Two separately implemented evaluators agree on the named cases; this is not complete semantic equivalence.                                               |
| Repeated evaluation by the same Echo evaluator                                                                | Deterministic self-validation                                                                                         | The implementation is repeatable for the tested basis and inputs; it is not an independent implementation.                                               |
| Receipt/WAL round-trip and fresh-host reconstruction                                                          | Deterministic self-validation                                                                                         | Retained bytes reconstruct the same installed and execution identities without callbacks.                                                                |

A separate verifier crate is not, by itself, independently implemented
conformance evidence. This decision does not claim formal refinement, complete
semantic equivalence, a clean-room Echo interpreter, or a proof that all
possible `ReplaceRange` inputs agree with the legacy planner.

The differential corpus counts as independently implemented evidence only
when its oracle path does not invoke the new program evaluator or generated
program implementation. Shared fact schemas and comparison codecs must be
named; agreement is asserted at the canonical output and patch boundary, not
by comparing two wrappers around the same algorithm.

## Jurisdiction

- **Jedit** owns `ReplaceRange.edict`, the rope-law resource, fact schemas,
  codec/identity law, operation semantics, and the differential oracle corpus.
- **Edict** owns source parsing, typing, canonical meaning, Core, generic
  resource binding, target invocation, package generation, clients, and
  provenance orchestration.
- **Echo's target implementation** owns the target profile, target lowering,
  structurally separate target verification, program ABI, and deterministic
  low-level intrinsic profile.
- **Echo's runtime** owns package and invocation admission, byte corroboration,
  installation, private evaluation, footprint enforcement, scheduling,
  exact-basis commitment, receipts, WAL, retention, and recovery.
- **Current Jedit/Jim integration** may provide canonical events, known basis,
  typed input, and external capabilities. It may not provide semantic execution
  or Echo authority.

## Rejected Alternatives

- Mutate provider v1 and describe it as provider v2.
- Treat matching callback claims as executable semantic evidence.
- Treat a program digest as an operation coordinate, invocation capability, or
  authority token.
- Install or invoke `EchoOperationProgramV1` without resolving it through an
  admitted executable-operation package.
- Generate an application matcher, executor, or footprint callback.
- Install a host-supplied `MutationPlan` or parent patch.
- Implement Jedit rope replacement as an Echo native intrinsic.
- Call Target IR executable merely because it names a target intrinsic.
- Commit a preparation after its exact evaluation basis changed.
- Describe a separate verifier crate as independently implemented without
  examining shared algorithms and dependencies.
- Build the general Cyber Kitten runtime before the operation seam works.

## Consequences

- Provider v1 remains byte- and API-stable compatibility infrastructure while
  migrations proceed; it is not the authority model for new executable
  operations.
- Echo gains one new program interpreter and installed-operation evidence
  category rather than a callback-bearing provider revision.
- Edict needs fixed-width and explicit-basis operation support plus exact
  executable-resource binding; this decision does not require a general
  recursive language.
- The first program format preserves contained-execution obligations without
  claiming durable child lanes, child ticks, wormholes, or universal WARP wire
  status.
- Execution evidence must distinguish package, program, admitted invocation,
  basis, declared and actual footprints, budget, scheduler composition, and
  execution-trace identities. A successful commit binds its patch and output;
  an obstruction binds a typed no-patch outcome instead.
- A receipt's program digest identifies the executable meaning used only in
  the context of the admitted operation and package that bind it; it does not
  retroactively make the program independently invocable or authoritative.
- A changed parent basis invalidates a preparation; automatic rebasing and
  cross-basis transport remain separately witnessed future operations.
- The existing Jedit planner becomes migration evidence rather than production
  execution authority.

## Non-Goals

- Cyber Kitten syntax or runtime;
- `TextWindow` or other optic migration;
- observer routing or event delivery;
- durable child worldlines or child-local ticks;
- wormholes, holograms, or Continuum transport;
- external effect execution;
- arbitrary native plugins, WASM, or a general-purpose VM;
- broad application authorization;
- create, checkpoint, inverse, save, or causal-topology migration.

## Evidence Anchors

- [Edict Target IR is a non-executing target review surface](https://github.com/flyingrobots/edict/blob/da5da887c1fa089a3f82f4d29d0799eb6e155f31/crates/edict-syntax/src/target_ir.rs#L1-L5)
- [Edict Core carries lawpack imports and bounded intent structure](https://github.com/flyingrobots/edict/blob/da5da887c1fa089a3f82f4d29d0799eb6e155f31/crates/edict-syntax/src/core_ir.rs#L14-L39)
- [Edict's initial compiler supports only one parameter and `basis none`](https://github.com/flyingrobots/edict/blob/da5da887c1fa089a3f82f4d29d0799eb6e155f31/crates/edict-syntax/src/compiler.rs#L608-L644)
- [Echo provider v1 stores host executor and footprint hooks](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/provider_contract.rs#L35-L126)
- [Echo rewrite rules execute native matcher, executor, and footprint functions](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/rule.rs#L11-L103)
- [Current provider evidence binds package, Target IR, and scheduler rule](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/warp-core/src/provider_contract.rs#L1181-L1263)
- [Echo runtime coordinates have canonical identifier and counter representations](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/echo-runtime-schema/src/lib.rs#L82-L153)
- [Echo frontier evidence carries tick, state-root, and commit identities](https://github.com/flyingrobots/echo/blob/6615d3a97731a076fb4945bb6da083e82f55710d/crates/echo-wasm-abi/src/kernel_port.rs#L364-L387)
- [Jedit's current `ReplaceRange` semantic planner](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope.rs#L385-L475)
- [Jedit persistent-rope split and balance law](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope/tree.rs#L69-L193)
- [Jedit currently binds replacement to handwritten callbacks](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/contract.rs#L111-L142)
- [Jedit's TypeScript text-graph proposition set](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/src/domain/graph-rope-types.ts#L12-L21)
- [Jedit's native text-fact proposition set](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/records.rs#L7-L14)
