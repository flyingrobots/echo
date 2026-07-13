<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Runtime Constellation

Echo is a deterministic WARP runtime over witnessed causal history. This map
keeps repository names subordinate to that boundary and makes transitional
components carry an explicit exit condition.

## Ownership Map

| Surface                               | Durable owner               | Posture                                                 | Boundary                                                                                                                                  |
| ------------------------------------- | --------------------------- | ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Echo / `warp-core`                    | Echo                        | Keep cohesive and private                               | Causal admission, scheduling, settlement, receipts, readings, retention coordinates, WAL/WSC integration                                  |
| `echo-trace`                          | Echo                        | Keep only if wired to the canonical commit/WSC boundary | Ordered causal-delta chunks and trace receipts; never a second event-sourcing truth                                                       |
| `echo-file-aperture`                  | Echo                        | Keep                                                    | File-site identity, explicit basis, admission, stale-basis posture, and materialization evidence; never host path or filesystem ownership |
| `warp-geom`                           | Bunny                       | Transitional; extract then delete                       | Pure reusable math, geometry, queries, meshes, and graphics schemas move to Bunny                                                         |
| `warp-math`                           | Bunny and Echo adapters     | Transitional                                            | Reusable deterministic numerics move to Bunny; Echo retains causal use and compatibility adapters only where needed                       |
| `echo-scene-port`, `echo-scene-codec` | No Echo owner               | Retire after evidence extraction                        | Preserve numeric parity and CBOR security gates; discard the renderer-shaped protocol                                                     |
| `@echo/renderer-three`                | No Echo owner               | Retire with the scene lane                              | Three.js presentation is not Echo runtime substrate                                                                                       |
| `warp-wasm` WARP DRIVE scaffold       | No durable owner            | Retire after aperture conformance exists                | A synthetic query bridge must not masquerade as the filesystem contract                                                                   |
| WARP DRIVE                            | WARP DRIVE                  | External consumer                                       | POSIX/FUSE presentation and host I/O; consumes a versioned Echo file-aperture contract                                                    |
| Continuum                             | Continuum                   | External protocol                                       | Lawful causal-history exchange; transport arrival is not Echo admission                                                                   |
| Wesley / Edict                        | Their compiler repositories | External compilers                                      | Authored contracts and rules lower into generated Echo packages; application nouns remain outside Echo core                               |
| `warp-ttd`                            | `warp-ttd`                  | External observer/tooling                               | Time-travel and debugger product concerns consume witnessed history; they are not Echo runtime authority                                  |

## `warp-core` Extraction Threshold

Size alone is not an extraction criterion. Keep optic admission, scheduler
authority, settlement, receipts, and runtime execution together until a leaf
has all of the following:

- a stable contract independent of engine internals;
- two real consumers with different ownership needs;
- executable compatibility and determinism witnesses; and
- no transfer of tick, admission, WAL, or recovery authority.

The first plausible leaf is the canonical WSC row/codec/storage boundary because
`NodeRow`, `EdgeRow`, and `AttRow` already have fixed representations and both
runtime and tooling need them. It should be extracted only when the second
consumer is real, not to make `warp-core` look smaller.

## Bunny Numeric Boundary

Bunny's deterministic scalar contract should be signed Q32.32 with raw `i64`
golden vectors and ties-to-even conversion, matching Echo's `det_fixed` lane.
Geometry extraction must preserve:

- inclusive AABB contact semantics;
- canonical pair ordering;
- deterministic broad-phase traversal; and
- explicit swept-bound sampling rules.

Pure vectors, matrices, quaternions, transforms, AABBs, overlap, raycast, sweep,
contact, broad-phase, mesh, and graphics-schema concerns belong in Bunny. Echo
ticks, worldlines, causal bases, admissions, receipts, retention, and
provenance do not.

The current `echo-wasm-abi` float-to-fixed conversion truncates while the
Q32.32 lane uses ties-to-even. That mismatch must be resolved with cross-language
golden vectors before calling a Bunny extraction parity-complete.

## Scene Evidence Exit Conditions

Deleting the scene lane must not delete its useful gates:

- Move the Rust/JavaScript numeric parity harness to fixed Q32.32 vectors owned
  by Bunny or the temporary deterministic-math boundary.
- Move CBOR count, trailing-byte, truncation, version, and discriminant security
  cases to the live `echo-wasm-abi` boundary.
- Replace unseeded random scene fixtures with checked-in deterministic vectors.

Once those witnesses no longer import scene types, the scene crates and
renderer have no Echo-owned reason to remain.

## Trace Honesty

The trace source of truth is the canonical stream of WSC row deltas. Roaring
selectors are indexes and rectangular prover traces are projections. A disabled
trace sink must return explicit disabled/no-receipt posture; an all-zero digest
must never be presented as proof.
