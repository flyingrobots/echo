<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# warp-core

_Map the implemented Rust kernel to Echo's Paper VII vocabulary: carrier state, settlement, retained patches, and observer-facing boundaries._

Legend: KERNEL

Depends on:

- [Echo Runtime Model](../architecture/outline.md)
- [WARP Rewrite Scheduler](scheduler-warp-core.md)
- [WARP Tick Patch](warp-tick-patch.md)
- [Merkle Commit](merkle-commit.md)

## Why this packet exists

`warp-core` is where Echo's runtime claims are made concrete. It stores carrier state, applies rewrite settlement, emits replayable patch artifacts, and computes hashes that later observers use for replay and audit.

## Human users / jobs / hills

Human users need one grounded map from research terms to Rust surfaces.

The hill: a maintainer can answer "where is that runtime concept implemented?" without reading every crate.

## Agent users / jobs / hills

Agent users need a stable public surface for tests, tools, and higher layers.

The hill: an agent can import `warp-core`, build a state, enqueue rewrites, commit a tick, and inspect the snapshot, receipt, and patch.

## Decision 1: `WarpState` is the carrier

The runtime state is a two-plane WARP: `stores: BTreeMap<WarpId, GraphStore>` plus `instances: BTreeMap<WarpId, WarpInstance>`. Each `GraphStore` contains skeleton nodes/edges and attachment values for one instance.

## Decision 2: Attachments are explicit plane values

Attachment values are typed atoms or explicit descent links. The hot path does not decode atom bytes by default. Descended attachment semantics are expressed through portals and descent-chain footprints.

## Decision 3: Rules settle through footprints

Rules provide matcher, executor, footprint function, stable rule id, and conflict policy. The engine enqueues candidates, drains them in canonical order, reserves declared footprints, and records accepted/rejected outcomes in the tick receipt.

## Decision 4: Commits emit three boundary artifacts

`commit_with_receipt` returns `Snapshot`, `TickReceipt`, and `WarpTickPatchV1`. These are retained witnesses. Observation layers may read them, but do not redefine the commit.

## Decision 5: Public ids are stable byte identities

Core ids are 32-byte values. Instance-scoped keys combine a warp id with a local node or edge id. Canonical ordering treats ids as raw byte strings.

## Implementation map

| Concept         | Primary surface                                    |
| --------------- | -------------------------------------------------- |
| carrier storage | `crates/warp-core/src/graph.rs`                    |
| attachments     | `crates/warp-core/src/attachment.rs`               |
| ids and keys    | `crates/warp-core/src/ident.rs`                    |
| footprints      | `crates/warp-core/src/footprint.rs`                |
| scheduler       | `crates/warp-core/src/scheduler.rs`                |
| engine          | `crates/warp-core/src/engine.rs`, `engine_impl.rs` |
| snapshots       | `crates/warp-core/src/snapshot.rs`                 |
| tick patches    | `crates/warp-core/src/tick_patch.rs`               |
| worldlines      | `crates/warp-core/src/worldline.rs`                |
