<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# jedit Optic Intent / Observation Handoff

- Lane: `up-next`
- Legend: `PLATFORM`
- Rank: `1`

## Why now

`jedit` now has a real authored GraphQL contract for its hot-text boundary,
including:

- mutation operations
    - `createBufferWorldline`
    - `replaceRangeAsTick`
    - `createCheckpoint`
- a canonical read operation
    - `worldlineSnapshot`

Wesley can generate TypeScript and Zod operation registries from that
contract, and `jedit` now consumes those registries in its app-owned adapter.

The next blocker is not in `jedit`. It is in the substrate handoff model.

Echo's current public wasm/kernel boundary exposes:

- generic intent ingest
- generic observation
- neighborhood publication
- settlement publication

That is directionally correct, but the integration posture now needs to match
the optic model more explicitly:

- the application submits intent through an optic-shaped boundary
- Echo admits or rejects that intent against generic substrate truth
- Echo returns the deterministic result / receipt envelope for that intent
- the application then observes the resulting worldline state and projects that
  generic graph truth into app-specific nouns

What is missing is not a `jedit`-named Echo API. What is missing is the first
clean handoff that connects:

- Wesley-compiled app intents
- Wesley-compiled observer plans
- generic Echo intent ingest / deterministic result envelopes
- generic Echo observation and hosted observer lifecycle
- app-side projection over observed worldlines

## Hill

Define the first substrate handoff that lets a `jedit`-style application use
Echo through optics without:

- putting app-specific rewrite names on Echo's generic public API
- reopening host-authored native rewrite code
- forcing `jedit` to fake causal state changes entirely in app-local runtime

## Done looks like

- one Echo-facing design or spec note states the optic handoff explicitly:
    - app submits intent
    - Echo returns the deterministic result / receipt envelope plus a
      hologram/frontier handoff
    - app reads through a generic observer plan or observer handle
- the note explains where app-specific operation names live:
    - authored in the app contract
    - compiled by Wesley
    - encoded into generic substrate intents
    - never promoted to handwritten Echo public methods
- the note also explains where app-authored observer behavior lives:
    - authored as app observer spec
    - compiled by Wesley into generic observer plans
    - hosted by Echo without handwritten app callbacks
- one concrete seam is named for the first `jedit` hot-text operations:
    - create buffer worldline
    - replace range as tick
    - create checkpoint
    - read canonical worldline snapshot
- repo truth makes clear how those operations travel through:
    - Wesley-generated intent / codec artifacts
    - Wesley-generated observer plan / reading codec artifacts
    - generic Echo intent ingest
    - deterministic receipt / result envelopes
    - generic observation and observer hosting
    - app-side worldline projection

## Repo evidence

- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-wasm/src/lib.rs`
- `docs/design/0012-dynamic-footprint-binding-runtime.md`
- `docs/invariants/DECLARATIVE-RULE-AUTHORSHIP.md`
- `/Users/james/git/aion-paper-07/optics/warp-optic.pdf`
- `docs/design/0013-generic-observer-api-and-plan.md`
