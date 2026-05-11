<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0006 — Echo Continuum alignment

_Decide what Echo must change so Continuum tools can consume one honest shared
observer/debugger noun stack without flattening Echo’s runtime-specific truth._

Legend: [PLATFORM](../../method/legends/PLATFORM.md)

Depends on:

- [0004 — Strand contract](./0004-strand-contract.md)
- [0005 — Echo TTD witness surface](./0005-echo-ttd-witness-surface.md)
- external Continuum packets:
    - `0001` through `0015`

## Why this cycle exists

Echo is older than the current Continuum theory stack.

That history shows up in the repo:

- some Echo runtime nouns pre-date the later lane / braid / witness language
- the runtime schema freeze in `schemas/runtime/` explicitly defers Wesley-owned
  generation
- the browser/TTD bridge path was prototyped locally before `warp-ttd` became
  the explicit observer/control plane

Those are not mistakes. They are historical truth.

The problem is interoperability pressure. If Continuum tools must work against
Echo and `git-warp`, then Echo cannot rely on "adapter folklore" forever. It
has to publish the shared observer/debugger nouns honestly enough that Wesley
and `warp-ttd` can consume them without reinterpreting Echo from the outside.

This cycle answers one narrow question:

**What must change in Echo to align with Continuum, and what should stay
Echo-local?**

## Design decision

Echo should **not** be flattened into Continuum theory terms everywhere.

Instead, Echo should align by making a cleaner split between:

1. **Echo-local engine/runtime nouns**
2. **shared Continuum observer/debugger contract nouns**
3. **adapter-only temporary synthesis**

The goal is:

- keep Echo’s hot-runtime truths where they belong
- stop using host adapters as permanent normalization swamps
- publish the same top-level observer/debugger nouns that Continuum tools will
  also expect from `git-warp`

## What should stay Echo-local

These are real Echo-specific runtime truths and should not be promoted just
because they matter:

- `GlobalTick` as a scheduler-global cycle stamp
- `SchedulerStatus`
- ingress routing and admission policy (`IngressTarget`, `InboxPolicy`)
- `WriterHeadKey`
- head eligibility / disposition
- materialization/finalization law details
- runtime-local control-plane types that only Echo needs

These are part of the Echo engine. They may be surfaced to tools, but they are
not the minimal shared Continuum contract.

## What Echo should publish into the shared Continuum surface

Echo should converge on the same observer/debugger-facing noun stack that
Continuum tools will want from both engines.

The important shared targets are:

- lane / worldline / strand identity at the observer boundary
- coordinate / frame truth
- admission outcome kind
- neighborhood core
- reintegration detail
- receipt shell
- effect emission
- delivery observation
- eventually observer trace

The key idea is:

- the internal engine may stay Echo-shaped
- the published observer/debugger contract should not feel like a different
  religion from `git-warp`

That includes one shared lawful outcome family at the publication boundary:

- `Derived`
- `Plural`
- `Conflict`
- `Obstruction`

Echo does not need to make every local subsystem emit all four outcomes
immediately. It does need to publish the same top-level outcome kind whenever a
shared observer/debugger surface depends on that distinction.

## Current Echo misalignment

### 1. No first-class local site object

`0005` already established that Echo can ground a narrow site from:

- `ObservationArtifact`
- `HeadObservation` / `SnapshotObservation`
- `WriterHeadKey`
- strand metadata

But Echo still lacks a first-class local-site object carrying:

- participating lanes
- local outcome
- nearby alternatives

That means the current adapter path can publish only a singleton or narrow site
summary. It cannot honestly support the fuller neighborhood model Continuum now
expects.

### 2. Reintegration truth is present but still scattered

Echo has strong raw ingredients:

- `TickReceipt.entries`
- `TickReceipt.blocked_by(idx)`
- `TickReceiptRejection`
- `ProvenanceEntry.parents`
- `ProvenanceEntry.head_key`
- `FinalizeReport.errors`

But those pieces are not yet normalized into a first-class reintegration core.

Continuum now distinguishes:

- seam anchors
- compatibility obligations
- compatibility evidence

Echo still makes consumers reconstruct that split from multiple runtime objects.

### 3. Receipt shell is strong, but too easy to overuse

Echo is already rich in shell/explanation objects:

- `ProvenanceEntry`
- `FinalizedChannel`
- `CursorReceipt`
- `TruthFrame`
- `SchedulerStatus`

That is useful, but dangerous. Without a stricter boundary, these shell objects
can quietly masquerade as neighborhood or reintegration truth.

### 4. Runtime schema generation is still deferred

`schemas/runtime/README.md` says the right thing honestly: Echo’s runtime
fragments are authored locally today, and Wesley generation is deferred.

That was a reasonable posture while the runtime shape was being frozen. It is
not a stable end state if Echo is supposed to align with Continuum and Wesley.

### 5. Compile-time footprint safety does not exist yet

Echo has real runtime footprint enforcement and compliance reporting. That is
good and should stay.

But Continuum’s current proof target now requires a stronger claim for at least
one slice:

- rewrite implementations should fail to compile when they exceed the declared
  footprint

Echo cannot satisfy that today because the generated capability surface does
not exist yet.

### 6. Browser host bridge still carries too much compatibility residue

`ttd-browser` and `echo-session-proto` are already narrower than before, but
they still carry more compatibility-era surface than the final architecture
wants.

That is not just cleanup debt. It affects alignment, because the more Echo’s
bridge surface remains locally defined and compatibility-shaped, the harder it
is to pin one shared observer contract.

## Required changes

### A. Add a native local-site publication boundary

Echo needs a first-class, runtime-backed publication shape for the debugger’s
local site.

This does **not** require a perfect "all alternatives" object on day one. It
does require one honest native surface that stops making every consumer derive
site truth ad hoc from observations and receipts.

Minimum target:

- stable site identity
- primary lane/worldline/head coordinate
- explicit statement of whether the site is singleton/narrow or plural
- top-level lawful outcome kind
- enough lane participation data to support shared neighborhood nouns later

### B. Add a native reintegration detail boundary

Echo should stop forcing consumers to reconstruct seam truth from scattered
receipt/provenance/finalization objects.

It needs a first-class reintegration publication surface that normalizes:

- seam anchors
- compatibility obligations
- compatibility evidence

This can still be synthesized internally at first, but it should be published
as one runtime-backed boundary object rather than as adapter folklore.

### C. Keep receipt shell explicit and subordinate

Echo should continue to expose rich shell objects, but make the layering
explicit:

- neighborhood core does not come from shell
- reintegration detail does not come from shell
- shell refines those layers; it does not define them

### D. Split Echo-local schema from shared contract composition

Echo should keep owning Echo-specific engine-level GraphQL families such as:

- runtime identifiers and counters
- routing/admission policy
- playback control richness
- scheduler status and engine-specific control-plane details

But Echo should stop acting like those local families are the whole story for
cross-repo tooling.

Instead:

- Echo-local engine schema stays in Echo
- shared observer/debugger families should come from the Continuum/Wesley side
- Echo composes those generated/shared families with its own local runtime
  families

### E. Land one Wesley-generated proof slice in Echo

Echo does not need to migrate wholesale first.

It does need one narrow proof slice where:

- the family is authored once through Wesley
- Rust-facing artifacts are generated
- Echo uses those generated artifacts in real runtime code
- a valid rewrite compiles and runs
- an invalid rewrite fails to compile because of footprint-shaped capability
  boundaries

This is the bridge from "alignment doctrine" to "real alignment."

### F. Narrow the browser host bridge to the real contract boundary

`ttd-browser` should become the narrow Echo browser host bridge, not the place
where debugger/product semantics continue to accumulate.

`echo-session-proto` should keep only what the bridge path actually needs and
shed the legacy transport residue that no longer represents the future
architecture.

### G. Publish the same top-level debugger nouns expected from `git-warp`

Echo should not wait for `warp-ttd` adapters to invent the shared nouns on its
behalf forever.

At the observer/debugger boundary, Echo should aim to publish the same top
categories that `git-warp` must also eventually publish:

- frame / coordinate
- lane identity
- neighborhood core
- reintegration detail
- receipt shell
- effect emission
- delivery observation
- observer trace

The exact internal engine structures can differ. The published categories
should not.

## Sequencing

### Phase 1: publish truthful local boundaries

- native local-site boundary
- native reintegration-detail boundary
- explicit shell layering
- one shared lawful outcome mapping across tick, neighborhood, and settlement publication

### Phase 2: prove Wesley ownership on one slice

- one generated runtime family
- one compile-fail footprint proof
- one codec path

### Phase 3: finish the bridge cut

- narrow `ttd-browser`
- split `echo-session-proto`
- feed `warp-ttd` through the shared contract instead of local folklore

## Current implementation note

Echo now has the first truthful runtime mapping for the shared lawful outcome
family:

- `TickReceiptDisposition::Applied` => `Derived`
- `TickReceiptDisposition::Rejected(FootprintConflict)` => `Obstruction`
- `NeighborhoodSite::Singleton` => `Derived`
- `NeighborhoodSite::Braided` => `Plural`
- `SettlementDecision::ImportCandidate` => `Derived`
- `SettlementDecision::ConflictArtifact` => `Conflict`

That is enough to prove the top-level algebra in runtime truth. The remaining
Continuum cut is to carry this mapping into the shared generated family
boundary instead of leaving adapters to infer it.

Echo also now has a native projection from `NeighborhoodSite` into an
Echo-side `NeighborhoodCore` DTO. That closes the first local-site publication
gap: the runtime can publish the shared neighborhood-core shape directly,
rather than requiring demo or adapter code to synthesize it by hand.

## What does not need to change

Echo does **not** need to:

- stop being the hot runtime
- erase `GlobalTick` / scheduler-specific truths
- hide ingress/admission richness
- adopt every Continuum noun internally
- wait for perfect neighborhood plurality before publishing honest narrow site
  truth

Alignment does not mean losing engine specificity. It means publishing shared
observer/debugger truth at the right boundary.

## Done looks like

Echo is aligned with Continuum when:

1. Echo-local engine families are clearly separated from shared contract
   families.
2. Echo publishes neighborhood core, reintegration detail, and receipt shell as
   explicit boundary layers instead of adapter synthesis folklore.
3. One Wesley-generated proof slice compiles into Echo runtime code and proves
   compile-time footprint enforcement.
4. `warp-ttd` can consume Echo through the same top-level observer/debugger
   nouns it will also expect from `git-warp`.

At that point, Continuum tools can treat Echo as one engine over the shared
causal language instead of as a special case they must reinterpret every time.
