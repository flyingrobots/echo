<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Strand Runtime Graph Ontology

- Lane: `up-next`
- Legend: `KERNEL`
- Rank: `2`

## Why now

Echo already has the correct execution law for worldlines:

- one mutable frontier per worldline
- immutable graph truth beneath the frontier
- content-addressed ingress
- deterministic scheduler admission
- one `super_tick()` path for state change

What it still lacks is one explicit runtime graph ontology for strands.
Until that exists, strands keep drifting toward folklore:

- manual ticking language survives in the strand contract
- support pins risk becoming accidental truth stores
- braid and settlement lack stable graph-native nouns to attach to

## Hill

Land one explicit graph-native control ontology for strands so Echo can
say, without hand-waving, what a strand is, where its current state
lives, what exact basis it was forked from, and which writer heads may
author work for it.

## Done looks like

- `docs/design/0009-strand-runtime-graph-ontology.md` exists and
  defines the authoritative strand runtime graph schema.
- The packet freezes authoritative node types and edge types for
  worldlines, strands, fork bases, current portals, and writer heads.
- The packet states the exact traversal used to obtain current strand
  state from graph truth.
- Support pins and braid publication are explicitly marked as derived /
  cache objects in the first cut.
- Follow-on runtime work can implement strand truth without inventing
  a second execution model.

## Repo Evidence

- `docs/design/0004-strand-contract.md`
- `docs/design/0007-braid-geometry-and-neighborhood-publication.md`
- `docs/design/0008-strand-settlement.md`
- `docs/invariants/STRAND-CONTRACT.md`
- `crates/warp-core/src/strand.rs`
- `crates/warp-core/src/worldline_state.rs`
- `crates/warp-core/src/head.rs`
- `crates/warp-core/src/coordinator.rs`
