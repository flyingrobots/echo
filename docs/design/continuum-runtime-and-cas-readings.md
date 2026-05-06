<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Continuum Runtime And CAS Readings

_Align Echo with the Continuum/WARP optic doctrine without making Echo
subordinate to `git-warp` or treating cached graph-like state as substrate
truth._

Status: Design

Owner: Echo / WARP runtime

Scope: doctrine and implementation runway only

## Hill

Echo is a peer Continuum runtime implementation.

Echo stores, executes, admits, observes, exports, imports, and retains
witnessed causal history in its own runtime style. Echo may use `echo-cas`,
indexes, checkpoints, and cached materialized readings for performance and
revelation. Those retained or cached objects are not substrate truth.

The substrate is witnessed causal history:

- worldlines
- strands
- braids
- ticks
- receipts
- frontiers
- payload hashes
- witnesses
- suffixes
- admissions
- readings
- retained artifacts

State-like values are observer-relative readings over that history.

## Doctrine Correction

Older Continuum and Echo docs sometimes describe Echo as the hot side and
`git-warp` as the cold side of one shared runtime story. That language is now
too narrow.

The corrected doctrine is:

```text
Echo is one Continuum-speaking runtime.
git-warp is another Continuum-speaking runtime.
Continuum is the shared protocol, contract, admission, and witness language.
```

Hot, cold, durable, low-latency, browser-hosted, offline-first, or archival are
runtime postures. They are not protocol ontology.

Echo does not produce facts for `git-warp`. Echo and `git-warp` both produce,
admit, retain, and observe witnessed causal history. They interoperate by
exchanging protocol-shaped causal artifacts, not by sharing runtime internals.

The browser analogy is the useful one: Continuum is the shared protocol and
contract law; Echo and `git-warp` are independent implementations with their
own engines, caches, storage models, and developer surfaces.

## Echo As A Continuum Runtime

Echo owns its local runtime truth:

- scheduling and deterministic execution
- ingress and admission policy
- worldline, strand, and braid runtime structure
- tick admission
- receipt creation
- observer execution
- retained runtime artifacts
- `echo-cas` storage policy
- runtime-local indexes and caches

Continuum constrains Echo at the shared boundary:

- shared noun meanings
- authored contract families
- admission outcome families
- witnessed suffix and import/export shells
- observer-relative reading envelopes
- receipt and witness layering

Echo should consume and publish Continuum-compatible families where the surface
crosses repo, language, process, WASM, network, or tool boundaries. Echo should
not flatten its internal engine shape into Continuum terms everywhere.

## Echo As Deterministic Witness Engine

Echo's primary product is deterministic witnessed causal transition.

In older runtime language, a tick mutates state. Under the Continuum/WARP optic
doctrine, a tick:

```text
slices causal inputs -> applies deterministic law -> emits transition ->
witnesses what happened -> retains enough evidence
```

Echo should therefore optimize its public semantics around:

- receipts
- observers
- apertures
- readings
- admissions
- braids
- retained witnesses

Public APIs should move toward operations such as:

- tick or admit a witnessed transition
- observe through an aperture
- read a bounded slice
- braid or compare witnessed claims
- retain or reveal witness-backed artifacts

Low-level materialization remains useful, but it should not define the public
ontology.

## No Canonical Materialized-State Ontology

Echo may keep state-like values. Echo may cache them. Echo may materialize
them for observation, debugging, export, or recovery. The rule is naming and
authority:

- witnessed causal history is substrate truth
- cached materialized state is a retained reading or recovery aid
- graph-like data is an observer-relative chart over history
- observer payloads are readings, not the runtime itself
- checkpoints are retained bases, not universal truth objects

Echo can store graph-like readings, graph-oriented indexes, and materialized
graph caches. Those objects are legal when they are scoped to a causal basis,
observer law, projection, and retention purpose. They are not the canonical
graph.

No public API should imply that Echo owns one canonical graph or state object
that all observers secretly read.

## echo-cas Role

`echo-cas` is Echo's content-addressed retention layer. It may store:

- tick receipts
- witnessed transition records
- payload blobs
- witness shells
- suffix shells
- reading artifacts
- checkpoint bases
- index shards
- cached slice readings
- retained observer outputs

The CAS hash names bytes. The semantic key names the question those bytes
answer.

That means a cached reading needs both:

- a content hash for the retained artifact
- an honest causal or optic coordinate for lookup and invalidation

Example coordinate components:

- runtime or history identity
- worldline, strand, or braid identity
- frontier or antichain basis
- observer or optic aperture
- read intent
- entity or slice key
- witness set digest or witness refs
- reducer or admission law version
- projection version
- rights and budget posture

`echo-cas` may retain the answer. It must not become the semantic authority for
the question.

## Holographic Retention Pressure

Echo should assume memory and local disk are finite.

The answer is not to materialize a full graph state at every tick. Echo should
retain witnessed causal history and enough boundary artifacts to support
bounded replay, bounded reveal, and honest obstruction. Optics then read by
slicing the required causal history, lowering only the focused aperture, and
optionally retaining the emitted reading.

For example, an optic that asks for `x` at coordinate `n+2` should be able to
use an index to find the nearest retained basis that affects `x`, stream the
required causal slice, lower the value, and retain the answer under a semantic
read key such as:

```text
focus=x
coordinate=n+2
aperture=value
witness_basis=...
projection_version=...
reducer_version=...
```

The retained bytes may live in `echo-cas` under a content hash, but the lookup
key is the read identity. A later optic that asks the same question may reveal
the retained bytes directly. A different coordinate, aperture, witness basis,
projection version, reducer version, rights posture, or budget posture is a
different question even if it happens to emit identical bytes.

Indexes that make this fast are performance aids. They should be streamable and
should not assume the full graph, full provenance log, or full index can fit in
memory at once. If the necessary retained basis or causal evidence is no longer
available locally, the read must return an obstruction or a rehydration-required
posture rather than secretly materializing unrelated state or pretending a cache
hit answers a different question.

Cache pressure is storage policy:

- evicting a cached reading does not rewrite history
- evicting an index shard does not invalidate receipts
- deleting unpinned CAS cache bytes may make a fast reveal unavailable
- deleting required witness material requires either rehydration or obstruction
- durable archival policy is separate from the content hash itself

`echo-cas` implementations may use content-defined chunking to reduce storage.
For large blobs or retained readings, a CAS tier may split bytes into variable
chunks chosen by content, MIME type, layout hints, or storage policy; buzhash
chunking is one plausible implementation technique. This can deduplicate
repeated substrings or common retained regions across related readings.

Chunking policy must remain below causal semantics:

- chunk boundaries are storage layout, not read identity
- changing a chunker must not change Intent identity, tick identity, receipt
  identity, admission outcome, or replay result
- semantic references above CAS must still name contract/schema/type/layout
  information where that information is required
- canonical byte encodings used by Echo history remain canonical before bytes
  enter retention

## Cached Reading Invalidation

Cached readings are immutable answers at a named basis. Echo should not mutate
them in place.

A cache entry remains valid for the coordinate, witness set, reducer version,
projection version, and rights/budget posture it names. It is not valid for a
new live frontier unless the lookup can prove causal containment or an
equivalent witness relation.

Invalidation is therefore mostly selection, not deletion:

- a new tick creates a new frontier
- a new suffix admission creates a new witness basis
- a changed observer law changes the projection version
- a changed admission or reduction law changes the reducer version
- changed rights or budget posture changes the revelation basis

Echo may garbage collect unreferenced cache blobs. That is storage policy, not
truth revision.

## Live Tail And Identity Honesty

Echo has the same live-tail honesty problem as `git-warp`.

If Echo has:

```text
retained checkpoint or cached reading
+ live tail of ticks, suffixes, or admissions
```

then Echo must not return the checkpoint or cached reading hash as if it
identified the live result.

Honest options are:

- reduce the live tail under a bounded witness basis
- return a read identity that names checkpoint basis plus tail witness set
- return a slice hash or witness-set hash whose meaning is explicit
- fail closed with an obstruction or missing-basis posture

Hash names should stay precise:

- `stateHash` names a full materialized state only when that is honestly what
  was produced
- `readIdentity` names coordinate, aperture, intent, witness basis, and
  law/projection versions
- `sliceHash` names the emitted bounded slice
- `witnessSetHash` names the witnesses used

No Echo API should return a global-sounding hash for an aperture-local reading
or a stale checkpoint basis.

## TickReceipt As Holographic Witness

A `TickReceipt` is not just an execution log. It is the retained witness that a
local causal transition was admitted under a specific runtime basis and law.

It should be sufficient, at its intended purpose level, to support:

- replay
- audit
- observer explanation
- suffix export
- import admission
- retained reading provenance
- future proof lowering

This does not mean v17 or the next Echo slice implements IPA, SNARKs, or other
cryptographic proof systems. It means Echo should preserve enough witness
structure that future proof backends can lower retained holographic artifacts
without reinterpreting runtime logs.

## Holographic Boundary Artifacts

Holographic witness structure should not be tick-only.

Echo needs a family of witness-bearing boundary artifacts across the same
recurring act:

```text
slice -> lower -> witness -> retain
```

Examples:

- a tick slices local causal inputs and witnesses an admitted transition
- a merge slices compatible or competing histories and witnesses what was
  preserved, conflicted, or left plural
- a braid slices multiple lanes or strands and witnesses the plural weave or
  lowered reading
- an import slices external suffix evidence and witnesses the local admission
  outcome
- an observation slices history through an aperture and witnesses the emitted
  reading

Names may evolve, but the layers should stay separate:

- tick receipt
- braid shell
- merge witness
- import shell
- admission response
- reading artifact
- property certificate
- observer receipt

This packet does not require all of those product APIs now. It requires Echo
to avoid flattening them into one runtime log or one state snapshot.

## Observer And Optic API

Echo reads should move toward optics over worldlines, strands, and braids.

An optic read is:

```text
choose aperture -> slice causal history -> lower under law -> witness ->
retain if needed -> emit observer-relative reading
```

The public shape should make the distinction visible:

- `ObserverPlan` names the revelation discipline
- `ObserverInstance` owns runtime observer state
- `ReadingEnvelope` names emitted observer-relative readings
- witness refs and residual posture explain what supports or limits the read

Worldlines, strands, and braids are all lawful optic sources. They are not
different kinds of global state object.

## Bounded Replay And Reveal

Bounded residency in Echo means more than avoiding a large graph
materialization. It also means Echo should not require full worldline or
runtime state just to answer a lawful observation, verify a local transition,
or reveal a retained artifact.

Echo should be able to ask:

- which causal slice is required for this read?
- which receipt and retained inputs are required to verify this tick?
- which witness material justifies this property or entity reading?
- which residual remains outside the requested aperture?

Future receipts and retained artifacts should be able to point at:

- read sets
- write sets
- affected entity or property sets
- basis and frontier
- payload hashes
- retained witness handles
- compact index hints where useful

The goal is not to make every receipt huge. The goal is to make bounded replay
and bounded reveal possible without rediscovering witness material from a full
state scan.

## Aperture-Aware Execution Semantics

Observation is aperture-aware, and execution should preserve enough structure
to make that true later.

A tick, admission, import, or braid may need to distinguish:

- input aperture
- effect aperture
- retention aperture
- reveal aperture

Those apertures enable bounded debugging, bounded replay, partial transport,
rights-gated revelation, and local repair. They should not be collapsed into a
single "state changed" fact.

## Attachments And Recursive Optic Boundaries

WARP entities may carry attachments. Some attachments may themselves descend
into another recursive WARP.

Echo should therefore treat attachments as aperture boundaries, not as bytes
that are always loaded with the parent reading.

An attachment may be:

- a content blob
- a causal artifact
- a nested WARP coordinate
- a retained reading
- a receipt or witness
- a proof-bearing artifact in a future phase
- a foreign suffix shell

Attachment descent should be explicit:

```text
observe entity -> encounter attachment -> check rights/budget/law ->
emit nested reading or obstruction -> retain boundary witness
```

Default entity reads should return attachment references or posture, not
silently load recursive worlds.

## Admission Algebra

Echo should keep admission outcomes typed and explicit:

- `Admitted`
- `Staged`
- `Plural`
- `Conflict`
- `Obstructed`

`Admitted` is the local successful admission posture. Continuum publication may
also use `Derived` where the shared family vocabulary requires that word. The
important rule is that success, staging, preserved plurality, conflict, and
obstruction remain separate outcomes.

Echo must not collapse admission into:

- `Ok` versus `Err`
- latest-writer-wins
- boolean sync success
- string status fields
- hidden host-time ordering

## Witnessed Suffix Evaluator Fit

The witnessed suffix evaluator is an Echo-local admission classifier.

It fits this doctrine because it:

- evaluates one witnessed suffix request against local read-only evidence
- returns a typed admission outcome
- distinguishes obstruction, conflict, plurality, staging, and admission
- does not fetch remote state
- does not run a sync daemon
- does not mutate worldlines or scheduler state
- does not depend on `git-warp`

In Continuum terms, the evaluator is part of import/admission law for a
protocol-shaped causal artifact. Transport can call it later. Transport must
not become the place where admission law is invented.

## Public Optic API Versus Plumber And Debug API

Echo should separate blessed reads from operational inspection.

Public optic APIs:

- read witnessed causal history through an aperture
- return reading envelopes or read identities
- surface residual, plurality, conflict, or obstruction posture
- never hide full materialization as a fallback
- never claim a cached reading is substrate truth

Plumber and debug APIs:

- may inspect caches
- may create or repair indexes
- may prewarm retained readings
- may export diagnostic materializations
- may run expensive validation
- must be named as operational work

The rule is:

```text
Optic observes.
Plumber maintains.
Debug explains.
```

A Plumber or debug operation must not become a hidden fallback for a public
optic read.

## Avoid Host-Bag Abstractions

Echo should not respond to this doctrine by adding a giant generic runtime bag.

Avoid broad names and helper surfaces that hide the law:

- `RuntimeFacade`
- `ObservationManager`
- `GraphLikeRuntimeAdapter`
- `SyncHostContext`
- `UniversalMaterializer`

Prefer small typed surfaces whose names expose the layer:

- admission
- observation
- reading
- witness
- retention
- braid
- plurality
- obstruction

The goal is not abstraction volume. The goal is making the causal law
inspectable.

## LWW Posture

Last-write-wins is allowed only as a projection or admissibility law for a
specific observer reading where that law is named.

LWW is not substrate truth.

In Echo terms:

- witnessed facts remain retained
- conflicting or plural facts remain inspectable
- an observer may lower a property reading with an LWW-style projection
- the projection must not erase the causal evidence that made plurality or
  conflict possible

One-line rule:

```text
LWW is a lens, not reality.
```

## Continuum Interoperability

Echo and `git-warp` interoperate by exchanging Continuum-shaped causal
artifacts:

- suffix shells
- receipts
- witness refs
- payload refs
- frontier identities
- admission outcomes
- reading envelopes where appropriate

They do not exchange:

- runtime internals
- scheduler state
- `echo-cas` implementation details
- Git implementation details
- private graph caches
- materialized state as canonical truth

The shared law is witnessed causal admission. Each runtime remains free to
store, cache, index, schedule, and observe in the way that fits its purpose.

## Implementation Runway

### Step 1: Keep This As Doctrine

This packet aligns Echo's mental model first. It may accompany small,
test-backed production slices, such as the `witnessed_suffix` evaluator and
tests, when those slices need doctrine-level determinism, hash, and canonical
ordering guidance.

### Step 2: Audit Stale Temperature Language

Later doc work should update older Echo and Continuum wording that assigns Echo
and `git-warp` fixed hot/cold half-roles.

Replace that framing with sibling Continuum runtimes and optional runtime
posture.

### Step 3: Tie Suffix Admission To Continuum Families

The witnessed suffix evaluator should stay local, but its input and output
shape should remain mappable to Continuum suffix/import families.

Do not add transport until admission law remains boring and tested.

### Step 4: Define CAS Reading Keys

A future RED slice should define the smallest honest cache-key structure for
retained readings:

- coordinate
- aperture
- intent
- witness basis
- reducer version
- projection version
- posture fields
- content hash

Do not make `echo-cas` responsible for ontology.

### Step 5: Define Live-Tail Reading Identity

Before returning hashes for live readings, Echo should define how a reading
names checkpoint basis plus live tail witnesses. If that basis cannot be named
honestly, the read must fail closed or return an obstructed posture.

### Step 6: Add Optic APIs Only When Bounded Reads Are Clear

Do not add a broad public optic surface until Echo can name:

- the causal basis read
- the witness basis retained
- the projection law used
- the invalidation rule for any cached reading
- the failure posture when evidence is missing

### Step 7: Keep Recursive Attachments Explicit

Future attachment work should expose attachment refs and recursive aperture
descent explicitly. It should not make default entity reads load attachment
bodies or nested WARP readings.

## Non-Goals

This design does not include:

- a `git-warp` dependency
- a sync daemon
- a global graph API
- a global state API
- a universal runtime facade
- proof or IPA implementation
- full Continuum wire protocol implementation
- ABI redesign unless required by RED tests
- storage engine replacement
- full cache eviction policy
- hidden materialization fallback
- broad host-bag runtime abstraction
- implicit recursive attachment loading

## Acceptance Criteria For Future Slices

Future implementation slices should prove:

- Echo optic reads name their causal basis
- cached readings name their witness basis and projection law
- live-tail readings do not reuse stale checkpoint hashes
- missing evidence fails closed
- suffix admission remains typed and local
- attachments are revealed by explicit aperture descent
- no public read API treats cached state as substrate truth
- no API makes Echo subordinate to `git-warp`
- Plumber/debug operations are explicit and not hidden read fallbacks

## Practical Rule

When adding Echo runtime surface area, ask:

- Is this witnessed causal history?
- Is this an observer-relative reading?
- Is this a retained shell?
- Is this a cache or index for performance?
- Is this a Plumber/debug operation?

If the answer is unclear, split the type before shipping it.
