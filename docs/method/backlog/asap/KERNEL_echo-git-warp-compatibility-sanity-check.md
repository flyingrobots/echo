<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo and git-warp compatibility sanity check

A systematic review of where Echo and git-warp align, where they
diverge, and what needs to happen for the two substrates to share
a debugger, a protocol, and a schema compiler.

## Context

Echo (Rust, deterministic simulation) and git-warp (JS, CRDT graph DB
on Git) are both WARP substrates. warp-ttd is the shared debugger.
Wesley is the shared schema compiler. But the two substrates evolved
independently and have diverged in ways that affect protocol design,
tooling, and future feature work (especially strands/braiding).

This item exists because a deep-dive in April 2026 surfaced
significant gaps that need deliberate alignment, not accidental drift.

Status: active coordination backlog. This is a compatibility map, not an
implementation spec; keep it aligned with current Echo runtime facts before
using it to drive cross-repo protocol work.

Triage update: keep this card, but sharpen the next pass around causal-history
interchange. The live question is no longer just whether Echo and git-warp can
share a debugger, protocol, and schema compiler; it is whether they can share
witnessed causal history through Continuum transport without making either
runtime subordinate to the other.

## Where they align

### Causal model

- Both use Lamport-style logical ticks (not wall-clock time)
- Both track per-writer provenance
- Both produce deterministic replay from identical inputs
- Both use receipts as structured provenance (applied/superseded/redundant)

### Protocol surface

- Both are targets of the warp-ttd protocol
- Both are targets for Wesley-compiled protocol/schema surfaces. Echo currently
  has generated TTD protocol consumers plus local runtime schema fragments; not
  every runtime fragment is Wesley-generated yet.
- Both expose worldlines, lanes, and frame-indexed playback

### Terminology

- Worldline, tick, receipt, observer, aperture — shared vocabulary
- warp-ttd's glossary enforces this (tested in CI)

## Where they diverge

### Convergence model

|              | Echo                                                        | git-warp                                       |
| ------------ | ----------------------------------------------------------- | ---------------------------------------------- |
| Strategy     | Canonical order (one scheduler, one result)                 | CRDT convergence (OR-Set + LWW, commutative)   |
| Multi-writer | Explicit sync protocol, scheduler-mediated                  | Native, automatic convergence                  |
| Merge        | No CRDT merge API; base-worldline strand settlement exists  | CRDT rules resolve automatically               |
| Determinism  | Non-negotiable — identical inputs produce identical outputs | Deterministic given same patches in same order |

This is the fundamental divergence. Echo's determinism is
scheduler-enforced; git-warp's is CRDT-enforced. Both are
deterministic, but the mechanisms are incompatible.

### Strands and braiding

|              | Echo                                                                                | git-warp                                                    |
| ------------ | ----------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| Fork         | `ProvenanceStore::fork()` plus `StrandRegistry`                                     | `createStrand()` — pinned base observation + overlay writer |
| Strand type  | First-class `Strand` with base ref, child worldline, writer heads, and support pins | Full: base observation, overlay, intent queue, evolution    |
| Braid        | Support pins exist; full braid presentation/collapse is not implemented             | Read-only composition of support strands without collapse   |
| Merge back   | Base-worldline strand settlement exists; no general braid collapse                  | `braidStrand()` merges overlay into base graph              |
| Compare      | `StrandSettlement::compare` exists for base-worldline settlement                    | `compareStrand()` diffs strand vs any coordinate            |
| Intent queue | Not present                                                                         | Speculative intents queued but not committed                |

Echo now has initial strand lifecycle, support-pin, and base-worldline
settlement semantics. It still lacks git-warp's richer braid composition,
durable overlay, intent queue, and general comparison/collapse model.

### Tick model

|               | Echo                                                             | git-warp                              |
| ------------- | ---------------------------------------------------------------- | ------------------------------------- |
| Tick identity | `WorldlineTick` (per-worldline) + `GlobalTick` (scheduler cycle) | Lamport clock (per-writer, monotonic) |
| Scheduler     | First-class: states, modes, run IDs, work state                  | None — replay is on-demand            |
| Playback      | `PlaybackCursor` with 5 modes + SeekThen policy                  | Seek cache + named bookmarks          |

### Ingress routing

|                | Echo                                                                | git-warp                               |
| -------------- | ------------------------------------------------------------------- | -------------------------------------- |
| Model          | `IngressTarget` (3-variant union) + `InboxPolicy` (3-variant union) | Direct writer append                   |
| Head admission | `HeadEligibility` (Dormant/Admitted) + `HeadDisposition` (4 states) | No concept — all writers always active |

### Storage

|             | Echo                                                                                                       | git-warp                                        |
| ----------- | ---------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| Persistence | Runtime state is still in-memory/ephemeral; `echo-cas` exists but is not yet the runtime persistence layer | Git-native (durable, distributed)               |
| Checkpoints | `ProvenanceStore` checkpoint interface (wormhole-ready)                                                    | Git snapshots after N patches                   |
| CAS         | `echo-cas` crate exists; runtime persistence integration remains pending                                   | `git-cas` (content-addressed blob store on Git) |

### Effect pipeline

|            | Echo                                                     | git-warp                                            |
| ---------- | -------------------------------------------------------- | --------------------------------------------------- |
| Channels   | Typed with policies (StrictSingle/Reduce/Log) + reducers | Effect entities (nodes with `@warp/effect:` prefix) |
| Delivery   | Channel materialization + FinalizedChannel               | EffectPipeline with externalization policy + sinks  |
| Compliance | `echo-ttd` PolicyChecker with structured violations      | No compliance concept                               |

## Protocol gaps (warp-ttd)

The warp-ttd protocol was shaped by git-warp. Seven specific gaps
exist for Echo (detailed in warp-ttd backlog item
`PROTO_echo-runtime-schema-alignment`):

1. **Typed opaque identifiers** — protocol uses `String!`, Echo uses
   32-byte hash-backed scalars
2. **WorldlineTick vs GlobalTick** — protocol collapses both to `Int`
3. **Playback control** — protocol has 3 mutations, Echo has 5 modes
   plus SeekThen
4. **Ingress routing** — not present in protocol at all
5. **Head eligibility/disposition** — not present
6. **Scheduler introspection** — not present
7. **WriterHeadKey composite** — flattened to single string

## Schema compiler (Wesley)

Both substrates use Wesley for schema compilation, but:

- git-warp generates TypeScript types + Zod validators
- Echo generates Rust types via `echo-wesley-gen` and `echo-ttd-gen`
- warp-ttd has its own schema (`warp-ttd-protocol.graphql`)
- Echo has runtime schema fragments (`schemas/runtime/artifact-a through d`)
- These schemas are not yet coordinated

## What needs to happen

### Short term (coordinate, don't build)

- Verify whether Echo and git-warp can exchange witnessed causal history through
  Continuum transport while preserving each runtime's local admission law,
  receipt semantics, and storage model.
- Audit whether Echo's WASM ABI surface is sufficient for a warp-ttd
  host adapter (see `PLATFORM_echo-ttd-host-adapter` backlog item)
- Reconcile TTD protocol types — one schema, one source of truth
  (see `PLATFORM_ttd-schema-reconciliation` backlog item)
- Propose protocol extensions for Echo's richer runtime model
  (see warp-ttd backlog: `PROTO_echo-runtime-schema-alignment`)

### Medium term (design decisions needed)

- Build on the live-basis strand semantics captured in
  `docs/design/0010-live-basis-settlement-plan/design.md`
- Design compliance reporting as a protocol extension
  (see `KERNEL_compliance-protocol-envelope` backlog item)
- Evaluate `ttd-browser` crate overlap with warp-ttd's browser story
  (see Echo backlog: `PLATFORM_ttd-browser-host-bridge` and warp-ttd backlog:
  `DELIVERY_browser-ttd-adapter`)

### Long term (requires strand/merge design)

- Implement Echo strands with base observation pinning
- Design canonical merge semantics (not CRDT convergence)
- Expose parallel execution counterfactuals through the debugger
- Cross-substrate debugging (debug Echo and git-warp in the same
  warp-ttd session)
