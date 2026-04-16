<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# jedit Hot Text Runtime Host Surface

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

The next blocker is not in `jedit`. It is in Echo's host surface.

Echo's current public wasm/kernel boundary exposes:

- generic intent ingest
- generic observation
- neighborhood publication
- settlement publication

It does not expose an app-consumable hot-text runtime surface that could back
`jedit`'s current `HotTextRuntimePort` without inventing app-specific logic in
the wrong layer.

## Hill

Define the first consumable Echo host surface that can back a `jedit`-style
hot-text runtime without:

- putting app-specific rewrite names on Echo's generic public API
- reopening host-authored native rewrite code
- forcing `jedit` to fake an Echo integration on top of unrelated observer
  calls

## Done looks like

- one Echo-facing design or spec note states how a generated app contract is
  supposed to cross the host boundary
- the note explains where app-specific operation names live:
    - authored in the app contract
    - compiled by Wesley
    - hosted by Echo through a generic installed-schema / generated-surface
      model
- one concrete host-facing seam is named for the first `jedit` hot-text
  operations
    - create buffer worldline
    - replace range as tick
    - create checkpoint
    - read canonical worldline snapshot
- the seam is consumable from JS/WASM without editing `warp-core` for app
  names
- repo truth makes clear why the existing `dispatch_intent` / `observe`
  boundary is not yet enough for this use case

## Repo evidence

- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-wasm/src/lib.rs`
- `docs/design/0012-dynamic-footprint-binding-runtime.md`
- `docs/invariants/DECLARATIVE-RULE-AUTHORSHIP.md`
