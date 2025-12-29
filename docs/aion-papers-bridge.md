<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# AIΩN Foundations → Echo: Bridge

Last reviewed: 2025-12-28.

This doc maps the **AIΩN Foundations series** (“WARP Graphs”, Papers I–VI) onto the **Echo** repository as it exists today.

Goal: keep the repo’s *implemented* determinism contracts and its *spec narrative* aligned with the papers that motivated them.

## Scope / Sources Read

For background and public context:

- AIΩN Framework repo: <https://github.com/flyingrobots/aion>

Published paper links (DOIs):

- Paper I: <https://doi.org/10.5281/zenodo.17908005>
- Paper II: <https://doi.org/10.5281/zenodo.17934512>
- Paper III: <https://doi.org/10.5281/zenodo.17963669>
- Paper IV: <https://doi.org/10.5281/zenodo.18038297>
- Papers V–VI: not yet published (as of 2025-12-28).

Note: the TeX sources used to author Papers I–VI are maintained outside this repo and are intentionally not vendored into Echo.

## Terminology (WARP vs RMG)

The AIΩN Foundations papers standardize on **WARP graph** (Worldline Algebra for Recursive Provenance) as the public name for the substrate.

This Echo repo historically used the older name **RMG** / “recursive metagraph” in crate names, type names, and docs (e.g., `rmg-core`, `RmgFrame`, “RMG viewer”).
Those identifiers have now been mechanically renamed to **WARP** equivalents (e.g., `warp-core`, `WarpFrame`, `warp-viewer`), but you may still see “RMG” in older notes and historical commit messages.

**Project policy:**

- Prefer **WARP** terminology in human-facing docs going forward.
- When Echo intentionally deviates from the paper design (for performance, ergonomics, or game-engine concerns), we **must** document the deviation and rationale:
  - record it in `docs/decision-log.md`, and
  - reflect it here (so readers of the papers learn what changed and why).

Status note: the mechanical rename from `rmg-*` → `warp-*` has landed (crates + the session/tooling surface).
The session wire protocol prefers `warp_*` op strings and `warp_id`, but decoders accept legacy `rmg_*` / `rmg_id` as compatibility aliases.

## Backlog Mapping (Paper → Echo)

These tables are intentionally “backlog-driving”: they identify what exists today, what is missing, and where Echo has (or may later choose) a different path.

### Paper I — WARP as the state object

| Paper I concept | Echo status | Touchpoints (today) | Backlog / next step | Deviation notes |
| --- | --- | --- | --- | --- |
| WARP graph = atom **or** skeleton-with-attachments | Partial (skeleton + opaque payloads) | `crates/warp-core/src/graph.rs`, `crates/warp-core/src/record.rs` | Decide whether recursive “attachments are WARPs” become first-class types or remain a payload encoding convention | Current engine spike treats “attachments” as bytes; may be enough for game engines if payload schemas are disciplined. |
| Depth / finite unfoldings | Not implemented explicitly | (N/A; conceptual) | If observers/tools need “unfold to depth k”, define a canonical encoding for nested payloads and add tooling helpers | Might stay in the tooling layer, not the core engine. |
| Morphisms / category framing | Not implemented explicitly | (Docs only) | Identify which morphism fragments matter for engine/tooling APIs (likely: stable IDs + isomorphism criteria for hashing) | Echo currently uses hashes + canonical encodings as “practical morphisms”. |

### Paper II — Deterministic evolution (ticks, independence, receipts)

| Paper II concept | Echo status | Touchpoints (today) | Backlog / next step | Deviation notes |
| --- | --- | --- | --- | --- |
| Tick = atomic commit (all-or-nothing) | Implemented for the spike (`commit` finalizes tx) | `crates/warp-core/src/engine_impl.rs` | Make abort/stutter semantics explicit if/when partial failure exists (currently “reserve rejects conflicts”) | Echo currently models conflicts as “not reserved”; explicit abort receipts are a good future addition. |
| Independence via footprints (delete/use; read/write sets) | Implemented (expanded to nodes/edges/ports + factor mask) | `crates/warp-core/src/footprint.rs`, `crates/warp-core/src/scheduler.rs` | Ensure footprint semantics remain “Paper II compatible” as optimizations land (bitmaps/SIMD, etc.) | Echo adds boundary ports + factor masks for engine practicality; document as an extension of the footprint idea. |
| Deterministic scheduling via total key order (“left-most wins”) | Implemented (deterministic ordering + deterministic reserve filter) | `crates/warp-core/src/scheduler.rs` | Specify the canonical key format (what exactly is “scope”?); keep stable across releases | Echo’s key is currently (`scope_hash`, `rule_id`, `nonce`); may evolve, but must remain deterministic. |
| Tick receipts (accepted vs rejected + blocking poset) | Implemented (minimal receipt; poset pending) | `crates/warp-core/src/receipt.rs`, `crates/warp-core/src/engine_impl.rs`, `docs/spec-merkle-commit.md` | Extend receipts with blocking attribution (poset) + richer rejection reasons once conflict policy/join semantics land | Current receipt captures accepted vs rejected in canonical plan order; only rejection reason today is footprint conflict. |

### Paper III — Holography (payloads, BTRs, wormholes)

| Paper III concept | Echo status | Touchpoints (today) | Backlog / next step | Deviation notes |
| --- | --- | --- | --- | --- |
| Boundary encoding `(U0, P)` where `P` is tick patches | Implemented in spirit | `crates/echo-graph` (`Snapshot` + `Diff`), `crates/warp-viewer/src/session_logic.rs` | Decide whether Echo’s “Diff stream” is *the* canonical tick-patch format (or one of several observers) | Echo’s stream is a practical boundary artifact; may not capture full tick receipts yet. |
| BTR packaging (hash in/out + payload + auth tag) | Partially implemented (hashes + canonical encoding + checksums) | `crates/warp-core/src/snapshot.rs`, `crates/echo-session-proto/src/wire.rs` | Define an explicit “BTR” message/record type for archives and replication (and later signing) | Today: checksums protect packets; signatures are future work. |
| Wormholes (compressed multi-tick segments) | Not implemented | (Concept only) | Add “checkpoint/wormhole” support once payloads are structured enough to compress/skip while preserving verification | Might be a tooling/storage feature, not required for realtime gameplay. |
| Prefix forks (content-addressed shared history) | Partially implemented (parents in commit hash) | `crates/warp-core/src/snapshot.rs` | Implement branch storage / addressable worldline families (Chronos/Kairos) | Echo already has parents; higher-level branch mechanics are still in docs. |

### Paper IV — Observer geometry + Chronos/Kairos/Aion

| Paper IV concept | Echo status | Touchpoints (today) | Backlog / next step | Deviation notes |
| --- | --- | --- | --- | --- |
| Chronos/Kairos/Aion triad | Specced; partially embodied (epochs, hashes, streams) | `docs/architecture-outline.md`, `crates/echo-session-service` (`ts`), `crates/echo-graph` (`epoch`) | Implement explicit branch-event (Kairos) and possibility-space (Aion) APIs once branch tree lands | Echo can keep the triad as “design axes” even before full branch tree exists. |
| Observers as projections over history | Embodied as tools/viewers | `crates/warp-viewer`, `docs/book/echo/booklet-05-tools.tex` | Define a “small family of canonical observers” and their guarantees (hash checks, partial views, privacy scopes) | Game tools want fast observers; papers motivate explicit translation costs. |

### Paper V — Provenance sovereignty (ethics requirements)

| Paper V concept | Echo status | Touchpoints (today) | Backlog / next step | Deviation notes |
| --- | --- | --- | --- | --- |
| Capability-scoped observers + due process | Partially specced / stub hooks exist | `crates/echo-session-proto` (handshake `capabilities`), `docs/spec-capabilities-and-security.md` | Evolve the session handshake into a real capability system tied to observer access | Echo can ship “developer mode” first, but must document the intended governance boundary. |

### Paper VI — JITOS / OS boundary + JS-ABI syscall framing

| Paper VI concept | Echo status | Touchpoints (today) | Backlog / next step | Deviation notes |
| --- | --- | --- | --- | --- |
| JS-ABI as stable, language-independent framing | Implemented | `crates/echo-session-proto/src/wire.rs`, `crates/echo-session-proto/src/canonical.rs` | Keep the framing boring and stable; add capability negotiation/versioning as needed | Echo already matches the “boring but essential” framing objective. |
| WAL / epochs as temporal backbone | Partially implemented (monotonic `ts`, epoch stream discipline) | `crates/echo-session-service`, `crates/echo-graph` | Define durable WAL / archive format and its relationship to commit hashes and diffs | Echo can treat session streams as “live WAL slices” and add persistence later. |

## Paper I — WARP as the state object (graphs all the way down)

**Core idea:** a *WARP graph* is either an **atom** (`Atom(p)` for opaque payload `p`) or a **finite directed multigraph skeleton** whose vertices and edges carry attached WARPs.

**Relevance to Echo:**

- Echo’s “everything is a graph” story is Paper I’s substrate claim.
- Echo’s current engine spike (`warp-core`) implements a *flat* typed graph store (`GraphStore` with node/edge records + payload bytes).
- The WARP “attachments are themselves graphs” concept is currently represented *implicitly* (payload bytes can encode nested graphs / structured payloads), not as a first-class recursive structure in Rust types.

**Echo touchpoints:**

- `crates/warp-core/src/graph.rs` + `crates/warp-core/src/record.rs` are the current concrete “graph store” substrate.
- `crates/echo-graph` is the canonical *tool/wire* graph shape.

## Paper II — Deterministic evolution: ticks, footprints, and receipts

**Core idea:** define a deterministic, concurrent operational semantics at the level of a **tick**:

- Within a tick, commit a scheduler-admissible batch (pairwise independent by footprint discipline).
- **Tick confluence:** any serialization order yields the same successor (up to isomorphism).
- Deterministic scheduling comes from a deterministic total order on candidates (“left-most wins”).
- Optional **tick receipts** record accepted vs rejected candidates and *why* (a poset of blocking causality).

**Relevance to Echo:**

- Echo’s runtime determinism is largely “Paper II made executable”:
  - collect candidate rewrites,
  - sort deterministically,
  - accept a conflict-free subset,
  - apply in deterministic order,
  - produce a hash.

**Echo touchpoints:**

- Deterministic pending queue + drain ordering:
  - `crates/warp-core/src/scheduler.rs`
- Footprints + independence checks:
  - `crates/warp-core/src/footprint.rs`
- Transaction lifecycle + plan/rewrites digests:
  - `crates/warp-core/src/engine_impl.rs`

**Notable gap (intentional/expected):**

- Echo’s current engine exposes “plan_digest” / “rewrites_digest”, but does not yet expose a first-class “tick receipt poset” structure for debugging/provenance the way Paper II describes.

## Paper III — Computational holography: provenance payloads, BTRs, wormholes

**Core idea:** for deterministic worldlines, the interior derivation volume is recoverable from a compact boundary:

- boundary encoding = `(U0, P)` where `P` is an ordered list of tick patches (payload)
- BTR (Boundary Transition Record) packages boundary hashes + payload for tamper-evidence/audit
- slicing: materialize only the causal cone required for some value
- prefix forks: Git-like branching via shared-prefix dedupe under content addressing
- wormholes: compress multi-tick segments into a single edge carrying a sub-payload

**Relevance to Echo:**

- Echo already treats hashing and canonical encoding as “truth checks”.
- Echo’s session pipeline is essentially a practical “boundary stream”:
  - full snapshots + gapless diffs (tick patches) with optional state hashes.

**Echo touchpoints:**

- `state_root` + `commit_id` and canonical encoding:
  - `crates/warp-core/src/snapshot.rs`
  - `docs/spec-merkle-commit.md`
- Gapless snapshot/diff semantics + per-frame hash checks:
  - `crates/echo-graph`
  - `crates/echo-session-proto`
  - `crates/echo-session-service`
  - `crates/warp-viewer/src/session_logic.rs`

## Paper IV — Observer geometry, rulial distance, Chronos/Kairos/Aion

**Core idea:** observers are resource-bounded functors out of history categories; translation cost induces geometry (rulial distance).

This paper also formalizes the three-layer time model used throughout Echo docs:

- **Chronos:** linear time of a fixed replay path (tick index)
- **Kairos:** branch events / loci of alternative continuations
- **Aion:** the full possibility space (history category; “Ruliad” as a large disjoint union)

**Relevance to Echo:**

- Echo’s Chronos/Kairos/Aion language isn’t “theme”; it’s an architectural partitioning of:
  - replay time,
  - branching structure,
  - and the larger possibility space/tooling surface.

**Echo touchpoints:**

- Conceptual: `docs/architecture-outline.md` (temporal axes)
- Practical precursor: hash-checked replay streams (viewer) + deterministic encoding (proto)

## Paper V — Ethics: provenance sovereignty as a runtime requirement

**Core idea:** deterministic replay + complete provenance becomes a capability that can be abused; therefore a runtime must treat provenance access as governed.

Paper V extracts system-level requirements (examples):

- consent + revocation,
- capability-scoped observers and view access,
- sealing / selective disclosure,
- fork rights (and constraints),
- due-process override protocols.

**Echo touchpoints:**

- `docs/spec-capabilities-and-security.md` (security/capability design space)
- Session protocol’s explicit “capabilities” field (`HandshakePayload`) provides a concrete hook to evolve toward scoped observers.

## Paper VI — JITOS: OS boundary layer and JS-ABI as syscall framing

**Core idea:** build an OS whose primary artifact is lawful transformations (history), with “state” as a materialized view; introduce SWS (shadow worlds), WAL-backed epochs, deterministic collapse, and JS-ABI as a stable syscall framing.

**Echo touchpoints:**

- JS-ABI framing + canonical payload encoding + checksums:
  - `crates/echo-session-proto/src/wire.rs`
  - `crates/echo-session-proto/src/canonical.rs`
- Session hub as an early “daemon boundary”:
  - `crates/echo-session-service`
- Viewer/tooling as early “observer” implementations:
  - `crates/warp-viewer`

## Practical Alignment Notes (What to Keep in Sync)

- Terminology drift: “RMG” vs “WARP”
  - Papers use “WARP” as the public substrate name; Echo now uses `warp-*` naming in crates and the session/tooling surface.
  - Docs and historical artifacts may still mention “RMG”; keep this note (and `docs/decision-log.md`) explicit about why/when a deviation exists.
- Empty-digest semantics for commit metadata
  - The engine’s canonical empty *length-prefixed list digest* is `blake3(0u64.to_le_bytes())`.
  - Keep docs consistent because this changes commit identity.
- Receipts / traces
  - Paper II receipts and Paper III payload/boundary formats are natural next layers over `plan_digest`/`rewrites_digest` and session diffs.
