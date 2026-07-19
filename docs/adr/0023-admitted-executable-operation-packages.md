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

Echo will add an operation-oriented corridor with these provisional first-
version nouns:

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

### First-version execution obligations

Every `EchoOperationProgramV1` must declare and canonically bind:

- one operation coordinate and exact input, output, and obstruction schemas;
- one invocation-to-graph binding;
- an explicit parent causal basis;
- a typed graph and attachment schema;
- a closed declarative rewrite/evaluation program;
- deterministic rule and match selection;
- only versioned, digest-locked, deterministic low-level intrinsics;
- a delegated step, allocation, and output budget;
- a declared footprint law or ceiling;
- a result projection and typed obstruction projection;
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

`PreparedEchoOperationV1` and its eventual receipt evidence must bind:

- the exact Echo parent worldline, writer head, frontier/tick, commit or root,
  and application basis named by the invocation;
- the program and package identities;
- the canonical input digest;
- the delegated budget and consumed budget;
- the declared footprint identity;
- the actual read/write footprint derived during evaluation;
- the resulting patch digest;
- the typed output or obstruction identity;
- the private execution or trace identity.

Before commit, Echo must establish all of the following:

```text
current parent basis == evaluated parent basis
installed program identity == evaluated program identity
canonical input identity == evaluated input identity
actual footprint is permitted by the declared footprint contract
committed patch identity == evaluated patch identity
```

If the parent basis changed, the prepared operation is ineligible to commit and
must yield a typed basis-changed posture without applying any patch. Echo must
not silently rebase, retarget, revalidate, or transport the preparation to the
new basis. A new evaluation is a new witnessed attempt unless an existing,
separately admitted Echo composition rule explicitly proves otherwise.

An existing scheduler composition rule may combine independent preparations
evaluated from the same parent snapshot. That is not revalidation across a
changed basis; the composed tick remains bound to the common evaluation basis.

### Jedit rope-law closure

For the first `ReplaceRange` vertical, Jedit will use a digest-locked canonical
declarative executable semantic resource imported through a Jedit-owned Edict
lawpack. Campaign 1 will not expand Edict source until it can directly express
the complete recursive persistent-rope algorithm.

The authoritative semantic closure consists of:

```text
Jedit-owned ReplaceRange.edict
+ exact Jedit lawpack coordinate and digest
+ canonical declarative EchoOperationProgramV1 bytes
+ exact fact schemas, codec profile, identity domains, and obstruction map
```

The Edict intent remains the application-owned operation surface. Its canonical
meaning binds the lawpack resource and program digest. Changing the executable
resource changes the admitted semantic closure and package identity.

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
operation runs would turn Campaign 1 into a broad language and runtime design
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
enter the same logic through another crate or call path. The first vertical
will label evidence as follows.

| Check                                                                                                         | Evidence grade                                                                                                        | First-version claim                                                                                                                  |
| ------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Canonical decoding, schema closure, digest recomputation, supported instruction set, and static budget bounds | Deterministic self-validation                                                                                         | The exact bytes are internally well-formed under the installed ABI.                                                                  |
| Installed package/program/input identity and exact-basis equality before commit                               | Deterministic self-validation                                                                                         | Echo committed only the admitted preparation against its evaluated basis.                                                            |
| Actual footprint against declared footprint law or ceiling                                                    | Deterministic self-validation                                                                                         | The evaluated support stayed within the admitted footprint contract.                                                                 |
| Core to Target IR relation                                                                                    | Structurally separate verifier path                                                                                   | A separately invoked verifier path recomputed or checked the declared relation; separation alone is not implementation independence. |
| Target IR plus lawpack resource to `EchoOperationProgramV1` binding                                           | Structurally separate verifier path                                                                                   | A separately invoked target verifier checked exact program/resource/schema/profile correspondence.                                   |
| Generated codec and package golden bytes                                                                      | Independently implemented conformance evidence only when the comparison implementation shares no encoder/lowerer path | The checked finite vectors agree; this is not a proof over all values.                                                               |
| Program result and patch versus the existing handwritten Jedit planner over a frozen differential corpus      | Independently implemented conformance evidence for that finite corpus                                                 | Two separately implemented evaluators agree on the named cases; this is not complete semantic equivalence.                           |
| Repeated evaluation by the same Echo evaluator                                                                | Deterministic self-validation                                                                                         | The implementation is repeatable for the tested basis and inputs; it is not an independent implementation.                           |
| Receipt/WAL round-trip and fresh-host reconstruction                                                          | Deterministic self-validation                                                                                         | Retained bytes reconstruct the same installed and execution identities without callbacks.                                            |

A separate verifier crate is not, by itself, independently implemented
conformance evidence. The first vertical will not claim formal refinement,
complete semantic equivalence, a clean-room Echo interpreter, or a proof that
all possible `ReplaceRange` inputs agree with the legacy planner.

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

## Sequencing Consequence

Implementation proceeds in dependency order:

1. minimal Edict operation prerequisites;
2. hook-free Echo bounded-operation evaluation with a tiny generic program;
3. the real Jedit `ReplaceRange` semantic artifact and differential oracle
   corpus;
4. current Jim invocation and legacy `ReplaceRange` cutover.

Each stage remains a separately reviewed and merged campaign. The first real
vertical is complete only when this chain is executable:

```text
real Jim command
→ explicit basis-bearing generated invocation
→ admitted Edict-authored operation
→ Echo-owned deterministic evaluation
→ one committed buffer consequence or typed obstruction
→ receipt binding the executable semantics
```

Campaign 1 is bounded to the Edict capabilities required to type the real
operation surface, preserve exact fixed-width values and explicit basis,
canonically bind the lawpack program resource through Core and artifact
identity, and generate the operation-facing package/client inputs. It does not
add the Echo evaluator, author the Jedit rope program, or introduce general
recursion, iteration, graph-pattern syntax, observer syntax, or process
semantics.

## Rejected Alternatives

- Mutate provider v1 and describe it as provider v2.
- Treat matching callback claims as executable semantic evidence.
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
- Campaign 1 stays bounded: it adds fixed-width and explicit-basis operation
  prerequisites plus exact executable-resource binding, not a general recursive
  Edict language.
- The first program format preserves contained-execution obligations without
  claiming durable child lanes, child ticks, wormholes, or universal WARP wire
  status.
- Receipts must distinguish package, program, input, basis, declared and actual
  footprints, patch, output or obstruction, budget, and execution-trace
  identities.
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
- [Jedit's current `ReplaceRange` semantic planner](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope.rs#L385-L475)
- [Jedit persistent-rope split and balance law](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/rope/tree.rs#L69-L193)
- [Jedit currently binds replacement to handwritten callbacks](https://github.com/flyingrobots/jedit/blob/c70e12d73b4b00bc92412bab67e1761f7dd22f82/native/jedit-echo-host/src/contract.rs#L111-L142)
