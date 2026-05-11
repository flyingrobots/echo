<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# jedit -> Echo -> Continuum Stack Plan

Status: active recovery plan  
Scope: stack-wide sequencing across jedit, Echo, Continuum, Wesley, warp-ttd,
and git-warp.

This document records the current execution plan for getting jedit onto Echo
without letting Echo become a text editor runtime, graph database, sync daemon,
or application framework.

## Control Thesis

jedit is the product pressure. Echo is the executable runtime pressure.
Continuum is the shared protocol and contract membrane. Wesley is the compiler
and witness generator. warp-ttd and other tools consume the published observer
surfaces. git-warp follows once Echo proves the boundary.

The first serious milestone is not Continuum schema alignment. The first serious
milestone is a tiny local jedit-through-Echo walking skeleton:

```text
createBuffer("demo.txt")
replaceRange(basis = B0, start = 0, end = 0, text = "hello")
textWindow(basis = B1, start = 0, length = 5)
=> "hello"
```

That path must run through:

```text
Wesley-generated or fixture-generated EINT
-> Echo dispatch_intent
-> Echo admission/provenance
-> Echo observe(QueryView)
-> ReadingEnvelope + QueryBytes
-> jedit consumes result
```

No repo gets to invent truth in isolation. Every layer either supplies
executable pressure or codifies a proven boundary.

Continuum should publish battle-tested boundaries, not horoscopes. It should
codify the seam after Echo and jedit prove the local runtime path, not lead the
product path with speculative protocol architecture.

## Sacred First Slice

The sacred first slice is:

```text
one user
one file-like object
one basis
one edit
one bounded read
one explanation of why the read is lawful
```

Everything else earns its place by serving that slice.

For the first slice:

- use a fake/generated fixture artifact if Wesley generation is not complete;
- make the fixture artifact have the same shape Wesley must later produce;
- prove Echo can host that artifact before requiring perfect compiler output;
- keep Continuum alignment narrow and downstream of the proven seam.

The fixture artifact must provide:

- manifest or artifact id;
- operation id;
- canonical vars bytes;
- declared footprint;
- mutation handler;
- query handler;
- artifact identity.

## Stack Witness 0001

Create a canonical cross-repo fixture:

```text
stack-witness-0001-jedit-file-history
```

Scenario:

```text
createBuffer(name = "demo.txt")
replaceRange(buffer, basis = B0, start = 0, end = 0, text = "hello")
textWindow(buffer, basis = B1, start = 0, length = 5)
=> "hello"
```

This witness should eventually produce or validate:

- EINT bytes;
- operation ids;
- canonical vars bytes;
- vars digest;
- footprint declaration hash;
- contract artifact id;
- mutation receipt;
- ReadingEnvelope;
- QueryBytes;
- debug rendering.

One story, many witnesses, no vibes.

## Non-Negotiable Runtime Rules

### Edit Intents Name Their Basis

A range edit must not be a naked byte replacement. It must name what it was
based on:

- target object id;
- basis ref;
- observed reading id or aperture id when available;
- range coordinate system;
- start;
- end;
- replacement bytes or text;
- contract artifact id.

An edit that does not name what it was based on is a stale-operation footgun.

### First Coordinate System Is Boring

Use UTF-8 byte offsets for Stack Witness 0001. Document that richer editor
coordinates are later work:

- Unicode scalar offsets;
- grapheme clusters;
- line and column;
- rope coordinates;
- syntax-aware spans.

Text range semantics are not universal. The first witness should choose a
small, explicit coordinate system and avoid pretending it is final.

### Echo Core Stays Noun-Clean

Echo core must not gain privileged app/editor nouns such as:

```text
jedit
rope
TextBuffer
ReplaceRange
TextWindow
Editor
Cursor
Selection
```

Those names are allowed in tests or fixtures only when clearly marked as
contract fixture vocabulary, for example:

```text
fixture_jedit_contract
test_text_window_contract
```

Echo hosts the contract. Echo does not become the contract.

### ReadingEnvelope Is Real Immediately

The first QueryView result must not be naked bytes. A minimum viable
ReadingEnvelope needs slots for:

- read identity;
- basis ref;
- observer plan or query id;
- contract artifact id;
- vars digest;
- aperture;
- payload digest;
- payload codec;
- witness refs or witness posture;
- budget posture;
- rights posture;
- residual or obstruction posture.

Some fields may be primitive or temporarily "not available", but the slots must
exist early.

### Receipts Carry Contract Law Identity

Every admitted contract mutation should leave evidence of:

- contract family id;
- schema or artifact id;
- operation id;
- operation version;
- footprint declaration hash;
- canonical vars digest;
- basis;
- admission outcome;
- receipt id.

Without this, replay, debug, and audit degrade into folklore.

### External Mutation Is Intent-Only

External jedit-facing paths must not call internal Echo services directly, such
as worldline creation, strand fork, settlement merge, support pinning, or
provenance mutation services.

The external shape is:

```text
dispatch_intent
observe
```

Internal services may remain implementation details behind admitted intent
handlers.

## Layer Roles

| Layer     | Role                                                                                 | Must not become                                                      |
| :-------- | :----------------------------------------------------------------------------------- | :------------------------------------------------------------------- |
| jedit     | Driving product fixture and first serious consumer                                   | A reason to put editor nouns into Echo core                          |
| Echo      | Deterministic witnessed runtime and admission/observation engine                     | A graph database, app framework, sync daemon, or text editor runtime |
| Continuum | Shared contract/protocol family owner                                                | A speculative ontology repo detached from runtime pressure           |
| Wesley    | Compiler, manifest, codec, witness, fixture generator                                | Semantic owner of shared protocol truth                              |
| warp-ttd  | Debugger/operator consumer of published receipts, readings, settlement, and suffixes | Hand-normalized adapter swamp                                        |
| git-warp  | Sibling runtime once the boundary is proven                                          | The durable half of Echo or the protocol authority                   |

## Immediate Stack Order

| Phase | Primary repo     | Result                                                                       |
| :---- | :--------------- | :--------------------------------------------------------------------------- |
| 0     | Echo             | Commit the architecture north star and clean branch inventory                |
| 1     | Echo             | Merge useful planning branches and delete stale branches                     |
| 2     | Echo/docs        | Add Stack Witness 0001: createBuffer -> replaceRange -> textWindow           |
| 3     | Echo             | Add RED tests for installed contract dispatch using fake/generated fixture   |
| 4     | Echo             | Add RED tests for QueryView observer bridge                                  |
| 5     | Echo             | Implement minimal installed contract host dispatch                           |
| 6     | Echo             | Implement minimal QueryView bridge returning ReadingEnvelope + QueryBytes    |
| 7     | Wesley           | Generate the fixture artifact for the same contract shape                    |
| 8     | jedit            | Consume generated helpers for createBuffer, replaceRange, and textWindow     |
| 9     | Echo + jedit     | Prove first end-to-end jedit-through-Echo slice                              |
| 10    | Continuum        | Publish or map the now-proven runtime-boundary surface                       |
| 11    | Wesley           | Compile Continuum runtime-boundary fixtures                                  |
| 12    | Echo             | Add boundary mappers to generated Continuum artifacts                        |
| 13    | Echo             | Add intent-only strand, braid, settlement, and inverse paths needed by jedit |
| 14    | jedit + Echo     | Prove braid projection and undo-as-inverse-history                           |
| 15    | Echo + echo-cas  | Add retention/streaming seams for bounded file windows                       |
| 16    | Continuum + Echo | Prove witnessed suffix export/import using the runtime-boundary family       |
| 17    | warp-ttd         | Inspect receipts, readings, braids, undo, and suffix outcomes                |
| 18    | git-warp         | Implement sibling runtime conformance after Echo proves the boundary         |

## Phase 0: Doctrine Checkpoint

Primary repo: Echo.

Immediate action:

```text
Commit the AGENTS.md architecture north-star update.
```

Suggested commit message:

```text
docs: record Echo architecture north star
```

Stop condition:

```text
Echo main says witnessed causal history is primary, readings are
observer-relative, mutation is explicit-base intent admission, transport is
witnessed suffix admission, and application nouns stay outside Echo core.
```

## Phase 1: Echo Branch Cleanup

Primary repo: Echo.

Recommended branch actions:

| Branch                                         | Action | Why                                                                                                                                     |
| :--------------------------------------------- | :----- | :-------------------------------------------------------------------------------------------------------------------------------------- |
| backlog/contract-hosted-file-history-substrate | Merge  | Best current roadmap for equipping Echo to support jedit through contracts, intents, readings, braids, inverse admission, and retention |
| docs/wip-branch-policy                         | Merge  | Useful process support for RED/GREEN work and intentional failing witnesses                                                             |
| cycle/0012-paper-vii-doc-alignment             | Delete | Stale and scope-contaminated; current docs supersede it                                                                                 |
| echo/docs-cleanup-20260127                     | Delete | Old broad cleanup that would regress the current no-graph/optic doctrine                                                                |

Stop condition:

```text
Echo main has the useful planning docs and no stale unmerged local branch noise.
```

## Phase 2: Stack Witness 0001 Spec

Primary repo: Echo docs.

Executable claim:

```text
The first stack witness names exactly one create/edit/read story and the
evidence each repo must eventually produce or consume.
```

Create:

```text
docs/design/stack-witness-0001-jedit-file-history.md
```

The witness must define:

- `createBuffer(name = "demo.txt")`;
- `replaceRange(buffer, basis = B0, start = 0, end = 0, text = "hello")`;
- `textWindow(buffer, basis = B1, start = 0, length = 5)`;
- expected payload bytes: `"hello"`;
- first coordinate system: UTF-8 byte offsets;
- required basis/read identity behavior;
- required contract artifact identity;
- required receipt fields;
- required ReadingEnvelope fields;
- allowed fixture shortcuts.

Stop condition:

```text
Future work can point to one tiny story instead of re-arguing the product
shape.
```

## Phase 3: Echo Installed Contract Dispatch RED

Primary repo: Echo.

Executable claim:

```text
Echo currently cannot admit a fixture-generated contract mutation through
dispatch_intent inside normal admission/provenance.
```

Use a fake/generated fixture artifact if Wesley is not ready. The fake must
match the intended generated host interface.

RED test:

```text
install tiny fixture contract handler
submit createBuffer or replaceRange EINT bytes through dispatch_intent
assert unsupported op obstructs before install
assert installed op is routed through admission/provenance
assert direct external service mutation is not required
assert receipt/provenance can carry contract artifact identity
```

Stop condition:

```text
The missing installed-contract dispatch seam is captured by a focused failing
test.
```

## Phase 4: Echo QueryView Observer RED

Primary repo: Echo.

Executable claim:

```text
Echo currently cannot route a fixture-generated QueryView request to a contract
observer and return ReadingEnvelope + QueryBytes.
```

RED test:

```text
install tiny fixture query observer
submit textWindow QueryView request
assert current behavior is unsupported or obstructed
assert expected GREEN behavior names basis, query id, vars digest, aperture,
payload digest, payload codec, and posture slots
```

Stop condition:

```text
The missing QueryView bridge is captured by a focused failing test.
```

## Phase 5: Echo Minimal Installed Contract Host

Primary repo: Echo.

Executable claim:

```text
A fixture-generated mutation can enter Echo through dispatch_intent and execute
inside admission/provenance through a generic installed contract host.
```

GREEN implementation:

| Component                         | Purpose                                |
| :-------------------------------- | :------------------------------------- |
| installed contract registry       | Map op id to fixture/generated handler |
| generic mutation handler trait    | Keep jedit types out of Echo core      |
| canonical vars decode             | Avoid caller-supplied JSON authority   |
| artifact/schema identity metadata | Receipts and readings name the law     |
| footprint authority from artifact | Caller cannot lie about footprint      |

Stop condition:

```text
Echo can admit createBuffer or replaceRange from the fixture artifact without
direct external service mutation.
```

## Phase 6: Echo Minimal QueryView Bridge

Primary repo: Echo.

Executable claim:

```text
A fixture-generated textWindow QueryView request routes to an installed
contract observer and returns QueryBytes with a real ReadingEnvelope.
```

GREEN implementation:

| Component                      | Purpose                              |
| :----------------------------- | :----------------------------------- |
| query op lookup                | Route to installed contract observer |
| canonical vars decode          | Deterministic read parameters        |
| coordinate resolution          | Explicit basis                       |
| ObservationPayload::QueryBytes | App-owned payload bytes              |
| ReadingEnvelope identity       | Observer-relative evidence boundary  |
| unsupported query obstruction  | No fake empty success                |

Explicit obstruction postures should include, or leave slots for:

- unsupported query;
- missing contract;
- stale basis;
- budget exceeded;
- rights limited;
- missing evidence.

Stop condition:

```text
A fixture textWindow query returns "hello" through Echo observation with
ReadingEnvelope metadata.
```

## Phase 7: Wesley Generates the Fixture Artifact

Primary repo: Wesley, consuming the Stack Witness 0001 contract fixture.

Executable claim:

```text
Wesley can generate the same artifact shape Echo already proved with the fake
fixture.
```

Required evidence:

| Evidence                                                 | Why                                               |
| :------------------------------------------------------- | :------------------------------------------------ |
| replaceRange has declared read/write footprint           | Echo must not trust caller JSON                   |
| textWindow has declared read footprint and aperture vars | Bounded read must be contract-visible             |
| generated EINT helper exists                             | jedit should not hand-roll bytes                  |
| generated QueryView helper exists                        | jedit should not bypass Echo observation          |
| manifest includes schema/artifact identity               | Echo receipts/readings can name contract identity |

Stop condition:

```text
Generated jedit fixture artifacts can replace the fake artifact in Echo's
walking skeleton.
```

## Phase 8: jedit Consumes the First Generated Helpers

Primary repo: jedit.

Executable claim:

```text
jedit can call generated helpers for createBuffer, replaceRange, and textWindow
without knowing Echo internals.
```

Stop condition:

```text
jedit can produce the same EINT and QueryView shapes used by the Echo walking
skeleton.
```

## Phase 9: First jedit-through-Echo Proof

Primary repos: jedit, Wesley, Echo.

Executable claim:

```text
jedit can create a buffer, apply a replaceRange edit, and read a textWindow
through Echo without direct mutation or raw state reads.
```

Target flow:

```text
jedit test
-> Wesley-generated createBuffer EINT
-> Echo dispatch_intent
-> receipt/provenance

jedit test
-> Wesley-generated replaceRange EINT
-> Echo dispatch_intent
-> receipt/provenance

jedit test
-> Wesley-generated textWindow QueryView
-> Echo observe
-> ReadingEnvelope + QueryBytes
```

Acceptance criteria:

| Assertion                                                 | Why                                  |
| :-------------------------------------------------------- | :----------------------------------- |
| Echo core contains no jedit, rope, buffer, or editor APIs | Preserve architecture                |
| Mutations enter through EINT and dispatch_intent          | Preserve intent discipline           |
| Reads enter through observation                           | Avoid hidden materialization         |
| Reading identity changes when basis or vars change        | Preserve honest observer semantics   |
| Same basis, query, and vars give stable identity          | Preserve deterministic read identity |

Stop condition:

```text
Stack Witness 0001 passes end-to-end through jedit, Wesley-generated helpers,
and Echo.
```

## Phase 10: Continuum Runtime Boundary Alignment

Primary repo: Continuum.

Suggested branch:

```text
schema/align-runtime-boundary-with-echo-jedit
```

Executable claim:

```text
Continuum runtime-boundary family names the shared surfaces Echo actually needs
to expose because Stack Witness 0001 proved them locally.
```

Required decisions:

| Current issue                                                            | Required decision                                                                |
| :----------------------------------------------------------------------- | :------------------------------------------------------------------------------- |
| Continuum schema has BaseRef while Echo uses ForkBasisRef                | Rename to ForkBasisRef or document BaseRef as a deliberate stable protocol alias |
| Runtime boundary has generic fields but no jedit pressure test           | Add fixture expectations driven by jedit/Echo use case                           |
| ReadingEnvelope lacks full posture detail in the current schema          | Decide whether v0.1 carries posture directly or references a posture object      |
| IntentEnvelope does not clearly mirror EINT v1                           | Preserve EINT compatibility or explicitly define the mapping                     |
| CausalSuffixBundle decouples source and target frontier                  | Keep this shape; it matches Echo's transport direction                           |
| Settlement basis report must match Echo live-basis settlement vocabulary | Align names and outcome categories with Echo                                     |

Non-goal:

```text
Do not design a universal protocol beyond what Echo and jedit need.
```

Stop condition:

```text
Continuum schema has a runtime-boundary family that can be compiled and mapped
onto Echo's current types without stale rename residue.
```

## Phase 11: Wesley Compiles the Continuum Boundary

Primary repo: Wesley, consuming Continuum schemas.

Executable claim:

```text
Wesley can compile the Continuum runtime-boundary family into deterministic
artifacts and fixture witnesses.
```

Work items:

| Slice                                                                                                              | Result                                                          |
| :----------------------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------- |
| Add runtime-boundary compile profile                                                                               | Generated Rust, TypeScript, manifest, and codec artifacts exist |
| Add fixture vectors for ForkBasisRef, ReadingEnvelope, WitnessedSuffixShell, CausalSuffixBundle, and ImportOutcome | Schema has concrete witness cases                               |
| Enforce canonical codec and registry ids                                                                           | No hand-authored shadow artifacts                               |
| Add drift-watch check                                                                                              | Continuum schema changes break generated fixture expectations   |

Stop condition:

```text
Continuum runtime-boundary family is compiled and fixture-witnessed, not merely
authored.
```

## Phase 12: Echo Boundary Mapping

Primary repo: Echo.

Executable claim:

```text
Echo can produce or consume generated runtime-boundary values for its current
admission, observation, settlement, and suffix surfaces.
```

Start with boundary mapping tests, not broad runtime refactors.

| Echo surface                 | Continuum family object                 |
| :--------------------------- | :-------------------------------------- |
| ForkBasisRef                 | ForkBasisRef or explicit protocol alias |
| tick/admission receipt       | TickResult                              |
| observation request          | ObservationRequest                      |
| reading result               | ReadingEnvelope                         |
| live-basis settlement report | SettlementBasisReport                   |
| witnessed suffix shell       | WitnessedSuffixShell                    |
| causal suffix export/import  | CausalSuffixBundle and ImportOutcome    |

Rule:

```text
Echo does not have to internally store generated Continuum structs everywhere.
It does need tested boundary mappers from real runtime objects to generated
Continuum artifacts.
```

Stop condition:

```text
Echo has a tested adapter layer from real runtime objects to generated
Continuum boundary artifacts.
```

## Phase 13: Intent-Only Runtime Topology Operations

Primary repo: Echo.

Executable claim:

```text
Externally visible strand, braid, support, settlement, and inverse operations
needed by jedit have intent paths.
```

Do not delete internal services. Wrap them behind intent handlers.

| Intent                    | Purpose                                              |
| :------------------------ | :--------------------------------------------------- |
| createContractStrand      | Create speculative lane through admission            |
| pinSupport / unpinSupport | Support geometry as witnessed operation              |
| createBraid               | Start generic ordered braid                          |
| appendBraidMember         | Add ordered member                                   |
| settleStrand              | Admit, obstruct, or produce conflict                 |
| settleBraid               | Collapse or preserve plurality                       |
| admitBraidProjection      | Make projection visible/admitted where policy allows |
| unapplyTick               | Ask contract for inverse intent and admit it         |

Stop condition:

```text
A jedit-style test no longer needs direct external calls for runtime topology.
```

## Phase 14: Generic Braid Substrate

Primary repo: Echo.

Executable claim:

```text
A contract-backed ordered braid can project edit members over a baseline and
expose complete, residual, plural, obstructed, or conflict posture.
```

Generic Echo nouns:

```text
Braid
BraidMember
BraidProjection
ProjectionDigest
BasisRevalidationPosture
```

Forbidden Echo core nouns:

```text
TextBraid
BufferBraid
RopeBraid
JeditBraid
```

Target jedit behavior:

```text
baseline = file worldline at B0
S0 forks from baseline
projection = baseline + S0
S1 forks from current projection
projection = baseline + S0 + S1
```

Stop condition:

```text
jedit fixture can append ordered edit members and observe braid projection
through QueryView.
```

## Phase 15: Inverse Admission for Undo

Primary repos: Echo, jedit, Wesley.

Executable claim:

```text
Typing hello then unapplying the third insert appends an inverse tick; the
original tick remains; the current reading is helo.
```

Target history:

```text
C0 add "h"
C1 add "e"
C2 add "l"
C3 add "l"
C4 add "o"
C5 inverse(C2)

reading = "helo"
```

Required assertions:

| Assertion                                        | Why                         |
| :----------------------------------------------- | :-------------------------- |
| Original C2 remains in provenance                | No history deletion         |
| Provenance length increases by one               | Undo is appended history    |
| Inverse receipt links to target receipt          | Preserve auditability       |
| Unmappable span obstructs                        | No fake undo                |
| Missing inverse fragment obstructs               | Retention truth             |
| Contract version mismatch obstructs or conflicts | Domain law identity matters |

Stop condition:

```text
jedit undo works as witnessed inverse admission, not rollback.
```

## Phase 16: Retention and Streaming Seams

Primary repos: Echo, echo-cas, jedit.

Executable claim:

```text
Large file readings and inverse fragments can be retained and read by bounded
aperture without full materialization as canonical truth.
```

Work:

| Surface                     | Requirement                                                    |
| :-------------------------- | :------------------------------------------------------------- |
| echo-cas                    | Remains opaque byte store                                      |
| semantic refs               | Carry contract, schema, type, layout, codec, and hash identity |
| textWindow query            | Can return visible range under budget                          |
| retained inverse fragment   | Resolves or obstructs                                          |
| cache                       | Never becomes canonical truth                                  |
| future WSC/Verkle/IPA slots | Preserved but not implemented unless needed                    |

Stop condition:

```text
A bounded jedit textWindow query returns only the requested aperture and
reports residual/budget posture honestly.
```

## Phase 17: Continuum Witnessed Suffix Exchange

Primary repos: Echo, Continuum, Wesley.

Executable claim:

```text
Echo can export and import witnessed suffix bundles conforming to Continuum
runtime-boundary artifacts.
```

Target flow:

```text
Echo export_suffix
-> WitnessedSuffixShell
-> CausalSuffixBundle
-> Echo import_suffix as intent
-> ImportOutcome
```

Required admission outcomes:

```text
ADMITTED
STAGED
PLURAL
CONFLICT
OBSTRUCTED
```

Required novelty postures:

```text
NOVEL
ALREADY_ADJUDICATED
SELF_ECHO
SUPPORT_SUPPLEMENT
ALTERNATE_SUPPORT_PATH
STATE_EQUIVALENT_DIFFERENT_WITNESS
```

Stop condition:

```text
Echo can round-trip a witnessed suffix bundle through the generated Continuum
family and classify the import outcome.
```

## Phase 18: warp-ttd Consumes Observer and Debugger Families

Primary repo: warp-ttd.

Executable claim:

```text
warp-ttd can inspect Echo-published receipts, readings, settlement plans, braid
projections, and import outcomes using generated Continuum/Wesley artifacts.
```

warp-ttd should become useful before full suffix exchange. After Stack Witness
0001, it should be able to show:

- operation admitted;
- basis used;
- contract law identity;
- receipt;
- reading identity;
- payload digest;
- why `textWindow` returned `"hello"`.

Debug views:

| View                    | Source family            |
| :---------------------- | :----------------------- |
| receipt list            | receipt-family           |
| delivery/witness detail | receipt-family           |
| settlement plan/result  | settlement-family        |
| neighborhood core       | neighborhood-core-family |
| reading envelope        | runtime-boundary-family  |
| suffix/import result    | runtime-boundary-family  |

Stop condition:

```text
A debugger can explain the jedit replaceRange, textWindow, and unapply flow
without private Echo-specific decoding.
```

## Phase 19: git-warp Sibling Runtime Conformance

Primary repo: git-warp.

Executable claim:

```text
git-warp can emit or consume the same Continuum runtime-boundary values for at
least one witnessed suffix exchange with Echo.
```

Target interop:

```text
Echo exports CausalSuffixBundle
git-warp imports or stages it
git-warp emits ImportOutcome

or

git-warp exports CausalSuffixBundle
Echo imports or stages it
Echo emits ImportOutcome
```

Stop condition:

```text
Continuum can honestly claim sibling-runtime witnessed suffix exchange, not
just authored schemas.
```

## Workstream Order

Use this concrete execution order:

```text
A. Echo branch cleanup and doctrine checkpoint
B. Stack Witness 0001 spec
C. Echo installed contract dispatch RED
D. Echo QueryView RED
E. Echo minimal fixture contract host
F. Echo minimal QueryView bridge
G. Wesley jedit fixture generation
H. jedit consumes generated helpers
I. jedit create/edit/read proof
J. Continuum runtime-boundary schema alignment around the proven seam
K. Wesley generated Continuum boundary fixtures
L. Echo boundary mapping tests
M. Echo intent topology operations
N. Echo generic braid substrate
O. jedit braid projection proof
P. Echo contract inverse admission
Q. jedit undo proof
R. Echo retention/streaming
S. Continuum suffix exchange proof
T. warp-ttd inspection proof
U. git-warp sibling proof
```

## Explicit Non-Goals

| Avoid                                             | Reason                                                    |
| :------------------------------------------------ | :-------------------------------------------------------- |
| Schema-first Continuum execution                  | It will drift away from jedit pressure                    |
| Full 16-phase plan as the day-to-day unit         | It is a map, not an executable slice                      |
| Starting with git-warp parity                     | Echo has the real implementation pressure right now       |
| jedit nouns in Echo core                          | Violates the runtime boundary                             |
| Braids before create/edit/read                    | Fancy footwork with no legs                               |
| BaseRef/ForkBasisRef as the first blocker         | Important schema cleanup, not the first runtime milestone |
| Verkle/IPA now                                    | Not needed for the first jedit proof                      |
| Dynamic plugin loading now                        | Static/generated contract hosting is enough               |
| Collaboration before one local file history works | Multi-user semantics should follow the local proof        |
| Rewriting all docs before implementation          | Executable witnesses matter more                          |
| Merging stale docs cleanup branches               | They regress current doctrine                             |
| CAS hash as reading identity                      | Bytes are not semantic truth                              |
| QueryView as just a query                         | It must emit observer-relative reading evidence           |

## Near-Term Commands, Conceptually

Do not treat this as an instruction to run blindly. This is the intended
sequence once the operator chooses to execute:

```text
1. Commit Echo AGENTS.md north-star update.
2. Merge Echo backlog/contract-hosted-file-history-substrate.
3. Merge Echo docs/wip-branch-policy.
4. Delete stale Echo branches.
5. Add Stack Witness 0001: createBuffer -> replaceRange -> textWindow.
6. In Echo, write RED tests for installed contract dispatch and QueryView.
7. Use a fake/generated fixture artifact first if Wesley is not ready.
8. Make Echo admit one contract mutation and return one ReadingEnvelope.
9. Make Wesley generate the real fixture artifact.
10. Make jedit consume it.
11. Then align Continuum around the proven boundary.
```

## Dependency Firewall

| Repo      | May depend on                                                                     | Must not depend on                               |
| :-------- | :-------------------------------------------------------------------------------- | :----------------------------------------------- |
| Echo      | generated contract host interfaces, opaque payloads, Continuum boundary artifacts | jedit implementation or editor runtime internals |
| jedit     | generated client helpers, Echo adapter                                            | Echo internals                                   |
| Wesley    | schemas, contracts, compiler fixtures                                             | Echo runtime internals                           |
| Continuum | shared schema families and proven boundary evidence                               | jedit-specific app implementation                |
| warp-ttd  | generated observer/debug artifacts                                                | private Echo decoding                            |
| git-warp  | Continuum families                                                                | Echo internals                                   |

## Golden Trace Option

Stack Witness 0001 may also produce a single trace file:

```text
001 createBuffer request
002 createBuffer receipt
003 replaceRange request
004 replaceRange receipt
005 textWindow request
006 ReadingEnvelope
007 QueryBytes
```

Every repo can consume or produce part of this trace. It becomes the stack's
Rosetta Stone.
