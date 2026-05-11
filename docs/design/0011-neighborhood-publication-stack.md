<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0011 - Neighborhood publication stack

Cycle: `0011-neighborhood-publication-stack`  
Legend: `PLATFORM`  
Source backlog item: `docs/method/backlog/up-next/PLATFORM_neighborhood-publication-stack.md`

## Sponsors

- Human: Runtime / host-boundary architect
- Agent: Host-adapter and debugger integration agent

## Hill

Echo has one explicit, inspectable explanation of how local-site truth moves
from runtime state to host-visible publication, including:

- what is authoritative admission truth
- what is commit-time emitted runtime truth
- what is read-time publication
- how `NeighborhoodCore` reaches the wasm/host boundary

The packet should answer this without requiring the reader to reconstruct the
stack from code comments, tests, and chat logs.

## Playback Questions

### Human

- [ ] Can I explain why `NeighborhoodCore` is published on demand rather than
      emitted automatically during every tick?
- [ ] Can I point to the exact difference between `ObservationArtifact`,
      `NeighborhoodSite`, and `NeighborhoodCore`?
- [ ] Can I explain how bytes reach the host without implying that
      neighborhood core is part of commit-time materialization?

### Agent

- [ ] Can I tell which API to call when I want raw observation, Echo-native
      local site truth, or the shared Continuum family shape?
- [ ] Can I explain the exact runtime path from `ObservationRequest` to
      CBOR-encoded `NeighborhoodCore` without guessing?

## Accessibility and Assistive Reading

- Linear truth / reduced-complexity posture: this packet should be readable as
  one left-to-right ladder: admission truth, observation truth, Echo-native
  site publication, shared family projection, ABI bytes.
- Non-visual expectations: all distinctions must be explicit in text. The
  packet should not depend on diagrams.

## Localization and Directionality

- Locale / wording / formatting assumptions: use stable ASCII-first nouns so
  the stack is reproducible in logs, tests, and host adapter docs.
- Logical direction / layout assumptions: this packet defines causal layering
  and boundary shape, not screen layout.

## Agent Inspectability and Explainability

- What must be explicit for agents: which objects are authoritative, which are
  derived, and which are boundary DTOs.
- What must remain attributable: if the host sees `NeighborhoodCore`, the
  system must name the kernel object and export path from which it came.

## Why this cycle exists

Echo now has enough real publication truth that the old “adapter will
eventually synthesize a narrow neighborhood core” story is no longer
accurate.

Repo truth now includes:

- admission-side `BoundedSite`
- raw observation via `ObservationArtifact`
- Echo-native local-site publication via `NeighborhoodSite`
- shared family projection via `NeighborhoodCore`
- wasm/ABI entrypoints for both `observe_neighborhood_site(...)` and
  `observe_neighborhood_core(...)`

Without one explicit packet for that stack, the system becomes hard to reason
about:

- hosts may assume `NeighborhoodCore` is a commit-time materialization channel
- maintainers may assume `ObservationArtifact` and `NeighborhoodSite` are just
  two names for the same thing
- debugger work may keep reintroducing adapter folklore

## Core distinction

Echo must keep three classes of truth separate.

### 1. Admission truth

This is where the engine decides lawful coexistence or obstruction.

Current noun:

- `BoundedSite`

This is authoritative for admission. It is not a host DTO and it is not
automatically revealed.

### 2. Commit-time emitted runtime truth

This is what Echo actually records or emits as part of execution:

- tick receipts
- provenance entries
- finalized channel outputs
- materialization frames

These arise because a tick committed or a finalization step happened.

### 3. Read-time publication

This is what Echo derives when a host asks a question about one local site:

- `ObservationArtifact`
- `NeighborhoodSite`
- `NeighborhoodCore`

These are not pushed automatically on every tick. They are produced on demand
from existing runtime truth.

## Publication ladder

The current stack is:

1. `BoundedSite`
    - admission-side focal closure
    - authoritative for coexistence law
2. `ObservationArtifact`
    - raw read / revelation artifact at a resolved coordinate
3. `NeighborhoodSite`
    - Echo-native publication of one local site
4. `NeighborhoodCore`
    - shared Continuum-family projection of that local site
5. ABI / wasm export
    - canonical bytes returned to the host

This ladder is deliberate.

It lets Echo:

- keep admission truth separate from observer publication
- keep Echo-native site publication separate from shared family DTOs
- keep receipt shell and reintegration detail separate from neighborhood core

## What each object is for

### `ObservationArtifact`

Use this when the host needs:

- resolved coordinate metadata
- declared frame/projection
- raw head or snapshot payload
- artifact hash and direct revelation context

It answers:

- what was observed?
- at what coordinate?
- under which observation contract?

It does **not** answer:

- which lanes define the local site?
- whether the site is singleton or plural?
- what the shared neighborhood-core family object should be?

### `NeighborhoodSite`

Use this when the host needs Echo-native local-site truth:

- site identity
- anchor coordinate
- participant set
- site plurality

It answers:

- which lanes participate in the currently observed local site?
- is the site singleton or braided?

It is still Echo-shaped:

- worldline ids remain typed kernel ids
- participant ticks remain worldline ticks
- plurality is `Singleton | Braided`

### `NeighborhoodCore`

Use this when the host wants the shared Continuum-family publication shape:

- string lane identities
- string site identity
- shared `AdmissionOutcomeKind`
- shared `NeighborhoodPlurality`
- shared participant roles
- summary text for debugger surfaces

It answers:

- what is the shared interoperable publication for this local site?

It is intentionally narrow:

- no reintegration detail
- no receipt shell
- no global nearby-alternative enumeration

## How publication happens

For the shared family boundary, the current runtime path is:

1. Host sends `ObservationRequest`
2. `warp-wasm` decodes canonical CBOR
3. `WarpKernel::observe_neighborhood_core(...)` resolves the request
4. `NeighborhoodSiteService::observe(...)` builds `NeighborhoodSite`
5. `NeighborhoodSite::to_core()` projects into `NeighborhoodCore`
6. `NeighborhoodCore::to_abi()` projects into the ABI DTO
7. `warp-wasm` returns canonical CBOR bytes to the host

This is read-time publication, not commit-time emission.

## Why Echo publishes both `NeighborhoodSite` and `NeighborhoodCore`

The system needs both levels.

`NeighborhoodSite` exists because:

- Echo needs one honest local-site publication object in runtime truth
- host adapters should not reconstruct participant sets from scattered objects

`NeighborhoodCore` exists because:

- shared family consumers should not need to understand Echo-native id types
- the shared Continuum neighborhood-core family has its own stable noun stack

So the duplication is not accidental. It is a controlled boundary split:

- Echo-native truth
- shared boundary truth

## What does **not** happen

Echo does **not** currently:

- auto-emit `NeighborhoodCore` as part of every tick
- store `NeighborhoodCore` as independent canonical runtime history
- treat neighborhood core as a replacement for `ObservationArtifact`
- use `NeighborhoodCore` as reintegration detail or receipt shell

If a host wants `NeighborhoodCore`, it must ask for it explicitly.

## Host guidance

Choose the read API by question.

### Use `observe(...)` when you need

- raw observation payload
- direct revelation context
- frame/projection specifics

### Use `observe_neighborhood_site(...)` when you need

- Echo-native local-site truth
- typed worldline / strand identities
- direct kernel publication

### Use `observe_neighborhood_core(...)` when you need

- the shared Continuum family shape
- host/debugger interoperability
- a stable publication object for cross-tool use

## Relation to reintegration and shell

Neighborhood core is not the whole debugger object model.

Still separate:

- reintegration detail
- receipt shell
- scheduler/provenance explanation
- global nearby-alternative enumeration

This packet therefore complements, but does not replace:

- `0005-echo-ttd-witness-surface`
- `0008-strand-settlement`
- `SPEC-0009-wasm-abi-v3`

## Current implementation note

The current repo truth is:

- `BoundedSite` exists as admission-side law
- `NeighborhoodSite` exists as runtime publication truth
- `NeighborhoodCore` exists as shared family projection
- `observe_neighborhood_site(...)` exists at the ABI boundary
- `observe_neighborhood_core(...)` exists at the ABI boundary

So the next host-side simplification is straightforward:

- consumers such as `continuum-demo` should stop synthesizing
  `NeighborhoodCore` in app code
- they should consume Echo's published neighborhood-core boundary directly

## Non-goals

- Define reintegration detail
- Define receipt shell semantics
- Define all nearby alternatives around a site
- Replace `ObservationArtifact`
- Replace `NeighborhoodSite` with only the shared family DTO

## Backlog Context

- `docs/design/0005-echo-ttd-witness-surface.md`
- `docs/design/0006-echo-continuum-alignment.md`
- `docs/design/0007-braid-geometry-and-neighborhood-publication.md`
- `docs/design/0010-bounded-site-and-admission-policy.md`
- `docs/spec/SPEC-0009-wasm-abi-v3.md`
