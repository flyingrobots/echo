<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Backlog Staleness Audit

This is a human triage layer over the generated METHOD DAG. Task truth still
belongs in `docs/method/backlog/**`, GitHub issues, design packets, and retros.
Use this audit to decide which unresolved cards to pull, rewrite, merge, or
close.

This snapshot was taken after completed backlog items were removed from
`docs/method/backlog/**`. The generated DAG now reports zero completed backlog
tasks. Generated `M###` IDs are not durable across backlog pruning; the source
path and task title are the durable handles.

Source snapshot:

- `docs/method/task-matrix.md`
- `docs/method/task-dag.dot`
- `docs/audits/suspicious-stuff.md`

## Staleness Labels

- `Current`: actionable as written or close enough to pull directly.
- `Current-after-tightening`: the concern is valid, but update wording before or
  during execution so it matches current doctrine.
- `Review-before-pull`: do a short stale-task audit before implementing.
- `Merge-or-close`: likely overlaps newer doctrine/cards; merge into the owning
  card or close with evidence.
- `Stale-close`: does not align with recent work; close/delete if no hidden
  dependency remains.
- `Future-park`: coherent enough to keep, but not useful as current frontier.

## High-Signal Cuts

- The stale inspector stream card was removed from the live backlog. The useful
  capability concern now lives in `M016`; the useful merge/settlement concern
  now lives in `M013`; debugger UI/protocol work belongs in `warp-ttd`.
- Rewrite or close Echo-core cards that still name Graft, direct editor
  hot-reload, or Shadow REALM as substrate work: `M028`, `M057`, `M058`, `M059`,
  and `M094`.
- Review Wesley/browser GraphQL, QIR, and typegen cards before pulling:
  `M040`, `M041`, `M043`, `M044`, and `M045`. Echo should own canonical
  Intent/observation boundaries, not a GraphQL-first runtime substrate.
- Treat `M016` and `M083` as consolidation candidates around one capability
  doctrine rather than separate drifting cards.

## Current Pull Bias

The least-stale open work is the deterministic/release-gate lane, the Echo
optics/reading envelope lane, the Wesley-to-Echo contract proof lane, and the
Continuum witnessed suffix lane.

Good current pulls include `M001`, `M002`, `M003`, `M004`, `M005`, `M006`,
`M007`, `M009`, `M010`, `M021`, `M024`, `M032`, `M034`, and `M042`.

## Inventory By Feature

### METHOD, Docs, And Process

- `M001` `Current` - Docs cleanup.
- `M031` `Review-before-pull` - Triage METHOD drift against `~/git/method`.
  Useful only if the external METHOD source is still intended to govern this
  repo.
- `M046` `Review-before-pull` - Wesley information architecture consolidation.
  May belong in Wesley once Wesley is the Rust library owner.
- `M047` `Future-park` - Wesley tutorial series and API reference. Keep behind
  ownership decisions.
- `M050` `Current` - First-class invariant documents.
- `M062` `Current` - Legend progress in `method status`.
- `M068` `Current-after-tightening` - Docs/logging improvements. Keep scoped to
  concrete defects.
- `M069` `Current` - Naming consistency audit.
- `M071` `Current` - Local rustdoc warning gate.
- `M073` `Current` - Current-head PR review / merge summary tool.
- `M074` `Current` - CI trigger rationalization.
- `M075` `Current` - Background Cargo lock isolation.
- `M076` `Current` - Small-commit pre-commit latency reduction.
- `M078` `Current` - PR review thread reply / resolution helper.
- `M079` `Current` - Shell script style / format lane.
- `M080` `Current` - Review-fix fast path for staged verification.
- `M081` `Current` - Pre-PR preflight gate.
- `M082` `Current` - Self-review command.
- `M083` `Current-after-tightening` - Pre-PR checklist and boundary-change
  policy. Tie it to current Echo doctrine rather than generic process prose.
- `M084` `Current` - Docs validation beyond Markdown.
- `M085` `Current` - Implementation-backed docs claims policy.
- `M086` `Review-before-pull` - Remove committed generated DAG artifacts. This
  conflicts with current use of committed generated METHOD artifacts.
- `M096` `Current` - Enforce Echo design vocabulary.
- `M117` `Future-park` - Cross-repo METHOD dashboard.
- `M127` `Future-park` - Extract METHOD crate to its own repo.
- `M128` `Current-after-tightening` - METHOD drift check as pre-push hook. Keep
  opt-in or clearly bounded.
- `M132` `Review-before-pull` - RED/GREEN cannot be separate commits. Reconcile
  with RED-first practice and the never-amend git rule.
- `M133` `Current` - `xtask main.rs` is a god file.

### CLI, Inspect, Verify, And Agent Surface

- `M005` `Current` - Config file support and shell completions.
- `M006` `Current` - Make decoder control coverage auditable.
- `M023` `Current-after-tightening` - Explicit Echo CLI and MCP agent surface.
  Keep it narrow; do not create a global mutable graph API.

### Determinism, Time, Hashing, And Release Gates

- `M003` `Current` - Deterministic trig oracle release gate.
- `M004` `Current` - CI determinism policy hardening.
- `M015` `Current-after-tightening` - SHA-256 to BLAKE3 migration spec. Frame it
  around canonical identity migration, not storage convenience.
- `M072` `Current` - Deterministic test engine helper.
- `M088` `Current` - SIMD canonicalization.

### Echo Optics, Observations, And Reading Envelopes

- `M012` `Current` - Contract-aware receipts and readings.
- `M014` `Current` - Parent drift and owned-footprint revalidation.
- `M032` `Current` - Reading envelope family boundary.
- `M090` `Current-after-tightening` - Hashable view artifacts. Reframe around
  `ReadIdentity`, witness basis, aperture, and projection/reducer versions.
- `M093` `Current-after-tightening` - Provenance as query semantics. Keep if
  rewritten as observer-relative reading/provenance query semantics.
- `M129` `Current-after-tightening` - Reading envelope inspector. Pull only
  after envelope families are clear enough to inspect.

### Wesley And Contract Hosting

- `M007` `Current` - Echo contract-hosting roadmap.
- `M010` `Current` - Wesley compiled contract-hosting doctrine.
- `M017` `Current` - Authenticated Wesley Intent admission posture.
- `M030` `Current-after-tightening` - jedit text contract MVP. Keep only as an
  example contract fixture, not Echo core ontology.
- `M037` `Review-before-pull` - Wesley go-public docs/CI. Confirm Echo versus
  Wesley ownership.
- `M038` `Review-before-pull` - Migration backfill script generation. Likely
  Wesley-owned unless Echo needs a host-side migration proof.
- `M039` `Review-before-pull` - Migration switch-over and contract validation.
- `M040` `Review-before-pull` - GraphQL operation parser for QIR. Likely stale
  in Echo if Wesley owns GraphQL/QIR parsing.
- `M041` `Review-before-pull` - SQL query plan generation from QIR. Very likely
  Wesley-owned or out of scope for Echo core.
- `M042` `Current` - Wesley to Echo toy contract proof.
- `M043` `Review-before-pull` - TypeScript type generation from Wesley IR.
- `M044` `Review-before-pull` - Zod validators from Wesley IR.
- `M045` `Review-before-pull` - CBOR bridge from TS types to WASM Rust. Keep
  only if it is a canonical adapter boundary, not causal ontology.
- `M063` `Review-before-pull` - Reconcile relocated Wesley Echo schemas.
- `M091` `Current-after-tightening` - Schema hash chain pinning. Align with
  artifact identity and read/receipt identity.
- `M092` `Review-before-pull` - SchemaDelta vocabulary. May be Wesley-owned.
- `M094` `Stale-close` - Shadow REALM investigation.
- `M095` `Future-park` - Multi-language generator survey. Probably
  Wesley-owned and not current Echo execution.

### Continuum, Suffix Admission, Import, And Interchange

- `M002` `Current` - Echo and git-warp compatibility sanity check.
- `M011` `Review-before-pull` - Compliance reporting as a TTD protocol
  extension. Check whether TTD is still the right host name/path.
- `M021` `Current` - Continuum proof family runtime cutover.
- `M026` `Current` - Echo / git-warp witnessed suffix sync.
- `M027` `Current-after-tightening` - Split `echo-session-proto` into retained
  bridge contracts vs legacy transport residue. Avoid broad host-bag
  abstractions.
- `M029` `Current` - Import outcome idempotence and loop law.
- `M060` `Review-before-pull` - git-mind NEXUS. Need evidence that this is
  still part of Echo's current integration map.
- `M061` `Review-before-pull` - Importer umbrella audit and close.
- `M116` `Future-park` - Continuum contract artifact interchange.

### Strands, Braids, Settlement, And Capability-Scoped Forking

- `M013` `Current` - Contract strands and counterfactuals.
- `M016` `Current-after-tightening` - Security/capabilities for
  fork/rewind/merge. Keep as the canonical Echo capability-law card; align it
  with the Optics capability model.
- `M028` `Stale-close` - Graft live frontier structural readings. Rewrite
  generically or close.
- `M099` `Future-park` - Parallel execution counterfactuals.

### Retention, CAS, Deep Storage, And Cached Readings

- `M022` `Current` - Contract artifact retention in `echo-cas`.
- `M024` `Current` - MemoryTier WASM compilation gate.
- `M025` `Current` - JS bindings for CAS store/retrieve.
- `M118` `Review-before-pull` - `Arc<[u8]>` to `bytes::Bytes` migration.
  Justify with measured storage/API friction.
- `M119` `Future-park` - `AsyncBlobStore` trait.
- `M120` `Future-park` - Enumeration and metadata API.
- `M121` `Current-after-tightening` - File-per-blob DiskTier implementation.
  Keep CAS bytes separate from ontology.
- `M122` `Current-after-tightening` - Tiered promotion/demotion. Must not
  affect causal identity.
- `M123` `Current-after-tightening` - Mark-sweep reachability analysis. Respect
  retained reading identity and witness needs.
- `M124` `Current-after-tightening` - Eviction policy and background sweep.
  Missing evidence must fail closed with obstruction.
- `M125` `Future-park` - Deep-storage wire protocol messages and binary
  encoding.
- `M126` `Future-park` - Deep-storage request/response protocol and
  backpressure.

### Browser, WASM, TTD, And Visualization Hosts

- `M008` `Review-before-pull` - Commit-ordered rollback playbooks for TTD
  integration.
- `M009` `Current` - Reconcile TTD protocol schemas with `warp-ttd`.
- `M018` `Current-after-tightening` - Canvas graph renderer. Update wording
  away from "static materialized reading" if it implies full hidden
  materialization.
- `M019` `Current-after-tightening` - Live tick playback and rewrite animation.
  Keep as observation/replay, not mutable runtime truth.
- `M020` `Current` - Node inspection panel.
- `M033` `Current` - Narrow `ttd-browser` into an Echo browser host bridge.
- `M034` `Current` - Wire Engine lifecycle behind `wasm-bindgen` exports.
- `M035` `Current` - Snapshot and `ViewOp` drain exports.
- `M036` `Current-after-tightening` - JS/WASM memory bridge and error protocol.
  Keep deterministic/canonical boundary constraints explicit.
- `M087` `Current` - Fuzzing the port.
- `M089` `Future-park` - Causal visualizer.

### Plugin, ABI, Sandbox, And Signing Security

- `M048` `Review-before-pull` - Rhai sandbox configuration. Confirm Rhai
  remains a live execution path.
- `M049` `Review-before-pull` - ViewClaim / EffectClaim receipts. Reframe
  through current receipt/reading doctrine if Rhai remains.
- `M051` `Review-before-pull` - Commit/manifest signing spec. Check old issue
  lineage.
- `M052` `Current-after-tightening` - Security contexts. Align with Optic
  capability, actor/cause, and admission law.
- `M053` `Review-before-pull` - FFI limits and validation.
- `M054` `Review-before-pull` - JS-ABI packet checksum v2. Confirm this is
  still the active JS/WASM boundary.
- `M055` `Current-after-tightening` - Provenance payload v1. Align with
  receipts, witness basis, and causal identity.
- `M056` `Current` - ABI nested evidence strictness.
- `M064` `Review-before-pull` - Key management doc.
- `M065` `Future-park` - CI sign release artifacts dry run.
- `M066` `Future-park` - CLI verify path for signatures.
- `M067` `Future-park` - CI verify signatures.
- `M077` `Current-after-tightening` - Feature-gate contract verification.

### Editor Hot Reload And Consumer-Specific Work

- `M057` `Stale-close` - Draft hot-reload spec. Do not make Echo core a
  file-handle/editor-hot-reload substrate; rewrite as adapter-only if needed.
- `M058` `Stale-close` - File watcher / debounce. Host-adapter concern, not
  Echo core.
- `M059` `Stale-close` - Hot-reload implementation. Host-adapter concern, not
  Echo core.
- `M070` `Future-park` - Reliving debugger UX design. Keep only as a consumer
  of optics/replay.

### Time Travel, Admission Inspector, And Rulial Diff

- `M100` `Future-park` - Rulial diff / worldline compare MVP.
- `M101` `Future-park` - Wesley worldline diff. Wait for contract query/read
  proof work.
- `M102` `Future-park` - Provenance heatmap.
- `M106` `Current-after-tightening` - Time travel core. Reframe around fixed
  ticks, playback coordinates, bounded reveal, and admitted timer history.
- `M107` `Future-park` - Reliving debugger MVP.

### Example Apps, Game Fixtures, And Course Material

- `M097` `Future-park` - Splash Guy course material.
- `M098` `Future-park` - Tumble Tower course material.
- `M103` `Future-park` - Splash Guy controlled desync.
- `M104` `Future-park` - Splash Guy lockstep protocol.
- `M105` `Future-park` - Splash Guy rules and state model.
- `M108` `Future-park` - Tumble Tower desync breakers.
- `M109` `Future-park` - Tumble Tower lockstep harness.
- `M110` `Current` - Replay-from-checkpoint convergence tests.
- `M111` `Current` - Replay-from-patches convergence property tests.
- `M112` `Future-park` - Tumble Tower stage 0 AABB.
- `M113` `Future-park` - Tumble Tower stage 1 rotation.
- `M114` `Future-park` - Tumble Tower stage 2 friction.
- `M115` `Future-park` - Tumble Tower stage 3 sleeping.
- `M130` `Future-park` - Splash Guy visualization.
- `M131` `Future-park` - Tumble Tower visualization.
