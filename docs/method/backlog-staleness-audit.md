<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Backlog Staleness Audit

This is a human triage layer over the generated METHOD DAG. Task truth still
belongs in `docs/method/backlog/**`, GitHub issues, design packets, and retros.
Use this audit to decide which backlog cards to pull, rewrite, merge, or close.

Source snapshot:

- `docs/method/task-matrix.md`
- `docs/method/task-dag.dot`
- `docs/method/stale-task-triage.md`

## Staleness Labels

- `Done-current`: completed and still aligned with current doctrine.
- `Done-superseded`: completed by explicitly superseding or retiring stale
  wording.
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

The fastest way to shrink the DAG is not to delete everything old. It is to
remove or rewrite cards whose vocabulary can currently steer the critical path
wrong.

- Close or rewrite the remaining `StreamsFrame`/`stream facts` work:
  `M175`, `M176`, `M177`, and `M178`. The useful concern is admission
  explanation, capabilities, and worldline/strand merge posture; the stale
  frame should not be treated as protocol truth.
- Keep `M173` and `M174` closed. They are resolved by fixed-timestep doctrine
  and modern optics/CAS retention doctrine.
- Keep `M064` closed. Wesley JSON IR v2 deserialization is superseded by the
  Wesley Rust library/generated Rust artifact direction.
- Rewrite or close Echo-core cards that still name Graft, direct editor
  hot-reload, or Shadow REALM as substrate work: `M065`, `M099`, `M100`,
  `M101`, and `M136`.
- Treat `M077` through `M082` as Wesley/browser-boundary review items, not
  obvious Echo-core work. Echo should own canonical Intent/observation
  boundaries, not a GraphQL-first runtime substrate.

## Current Pull Bias

The least-stale open work is the deterministic/release-gate lane, the Echo
optics/reading envelope lane, the Wesley-to-Echo contract proof lane, and the
Continuum witnessed suffix lane.

Good current pulls include `M002`, `M005`, `M007`, `M009`, `M014`, `M017`,
`M018`, `M038`, `M039`, `M057`, `M060`, `M069`, `M071`, and `M079`.

## Inventory By Feature

### METHOD, Docs, And Process

- `M001` `DONE` `Done-current` - Man page generation and README examples. Keep
  only as completion history.
- `M002` `OPEN` `Current` - Docs cleanup. Human triage already says keep.
- `M041` `DONE` `Done-current` - `xtask method close`. Keep as completion
  history.
- `M042` `DONE` `Done-current` - `xtask method drift`. Keep as completion
  history.
- `M043` `DONE` `Done-current` - `xtask method pull`. Keep as completion
  history.
- `M068` `OPEN` `Review-before-pull` - Triage METHOD drift against
  `~/git/method`. Useful only if the external METHOD source is still intended
  to govern this repo.
- `M083` `OPEN` `Review-before-pull` - Wesley information architecture
  consolidation. May belong in Wesley once Wesley is the Rust library owner.
- `M084` `BLOCKED` `Future-park` - Wesley tutorial series and API reference.
  Keep behind `M083`; do not pull until ownership is decided.
- `M087` `OPEN` `Current` - First-class invariant documents. Aligns with recent
  fixed-timestep and determinism work.
- `M104` `OPEN` `Current` - Legend progress in `method status`. Small METHOD
  observability improvement.
- `M110` `OPEN` `Current-after-tightening` - Docs/logging improvements. Keep
  scoped to concrete docs or logging defects.
- `M111` `OPEN` `Current` - Naming consistency audit. Useful with vocabulary
  enforcement.
- `M113` `OPEN` `Current` - Local rustdoc warning gate. Straightforward quality
  gate.
- `M115` `OPEN` `Current` - Current-head PR review / merge summary tool. Aligns
  with active PR workflow.
- `M116` `OPEN` `Current` - CI trigger rationalization. Still useful if backed
  by current CI evidence.
- `M117` `OPEN` `Current` - Background Cargo lock isolation. Still relevant to
  local workflow reliability.
- `M118` `OPEN` `Current` - Small-commit pre-commit latency reduction. Current
  developer-experience issue.
- `M120` `OPEN` `Current` - PR review thread reply / resolution helper. Aligns
  with current CodeRabbit/GitHub workflow.
- `M121` `OPEN` `Current` - Shell script style / format lane. Current because
  scripts enforce determinism and release gates.
- `M122` `OPEN` `Current` - Review-fix fast path for staged verification.
  Useful if it preserves evidence and does not skip gates.
- `M123` `OPEN` `Current` - Pre-PR preflight gate. Current workflow hardening.
- `M124` `OPEN` `Current` - Self-review command. Useful if it stays read-only
  and evidence-oriented.
- `M125` `BLOCKED` `Current-after-tightening` - Pre-PR checklist and
  boundary-change policy. Keep, but tie it to current Echo doctrine rather than
  generic process prose.
- `M126` `OPEN` `Current` - Docs validation beyond Markdown. Current after docs
  build and generated inventory issues.
- `M127` `OPEN` `Current` - Implementation-backed docs claims policy. Directly
  addresses overclaiming in stale cards.
- `M128` `OPEN` `Review-before-pull` - Remove committed generated DAG
  artifacts. This conflicts with current use of committed `task-matrix` and DAG
  artifacts; decide before acting.
- `M138` `OPEN` `Current` - Enforce Echo design vocabulary. Useful after recent
  vocabulary cleanup.
- `M159` `OPEN` `Future-park` - Cross-repo METHOD dashboard. Coherent, but not
  current Echo kernel/API work.
- `M169` `OPEN` `Future-park` - Extract METHOD crate to its own repo. Do not
  pull until Echo-local METHOD stabilizes.
- `M170` `OPEN` `Current-after-tightening` - METHOD drift check as pre-push
  hook. Built on `M042`; keep opt-in or clearly bounded.
- `M180` `OPEN` `Review-before-pull` - RED/GREEN cannot be separate commits.
  Needs reconciliation with the current "RED tests first" habit and "never
  amend" git rule.
- `M181` `OPEN` `Current` - `xtask main.rs` is a god file. Real maintainability
  issue in the METHOD/xtask lane.

### CLI, Inspect, Verify, And Agent Surface

- `M010` `DONE` `Done-current` - Bench subcommand. Keep as completion history.
- `M011` `DONE` `Done-current` - Inspect metadata and graph stats. Keep as
  completion history.
- `M012` `DONE` `Done-current` - Inspect attachment payload pretty-printing.
  Keep as completion history.
- `M013` `DONE` `Done-current` - CLI subcommand scaffold. Keep as completion
  history.
- `M014` `OPEN` `Current` - Config file support and shell completions. Human
  triage already says keep.
- `M015` `DONE` `Done-current` - Verify hash recomputation. Keep as completion
  history.
- `M017` `OPEN` `Current` - Make decoder control coverage auditable. Human
  triage already says keep.
- `M059` `OPEN` `Current-after-tightening` - Explicit Echo CLI and MCP agent
  surface. Keep the surface narrow; do not let it become a global mutable graph
  API.

### Determinism, Time, Hashing, And Release Gates

- `M003` `DONE` `Done-current` - 1-thread vs N-thread determinism harness.
  Keep as completion history.
- `M004` `DONE` `Done-current` - Snapshot/restore fuzz. Keep as completion
  history.
- `M007` `OPEN` `Current` - Deterministic trig oracle release gate. Human
  triage already says keep.
- `M009` `OPEN` `Current` - CI determinism policy hardening. Human triage
  already says keep.
- `M048` `OPEN` `Current-after-tightening` - SHA-256 to BLAKE3 migration spec.
  Keep if framed around canonical identity migration, not storage convenience.
- `M049` `DONE` `Done-superseded` - HistoryTime vs HostTime field
  classification. Closed into fixed-timestep invariant doctrine.
- `M050` `DONE` `Done-current` - TTL/deadline semantics are ticks only. Closed
  into admitted timer Intent doctrine.
- `M114` `OPEN` `Current` - Deterministic test engine helper. Aligns with the
  current release-gate direction.
- `M130` `OPEN` `Current` - SIMD canonicalization. Keep; this is determinism
  hygiene if SIMD remains in scope.

### Echo Optics, Observations, And Reading Envelopes

- `M019` `DONE` `Done-current` - Echo Optics ABI DTOs. Keep as completion
  history.
- `M020` `DONE` `Done-current` - Echo Optics adapter notes. Keep as completion
  history.
- `M021` `DONE` `Done-current` - Echo Optics API design. This is current
  doctrine.
- `M022` `DONE` `Done-current` - Echo Optics attachment boundary model. Keep as
  completion history.
- `M023` `DONE` `Done-current` - Echo Optics cached-reading identity tests.
  Keep as completion history.
- `M024` `DONE` `Done-current` - Echo Optics core nouns and IDs. Keep as
  completion history.
- `M025` `DONE` `Done-current` - Echo Optics dispatch intent model. Keep as
  completion history.
- `M026` `DONE` `Done-current` - Echo Optics example implementation. Keep as
  completion history.
- `M027` `DONE` `Done-current` - Echo Optics live-tail honesty tests. Keep as
  completion history.
- `M028` `DONE` `Done-current` - Echo Optics observe model. Keep as completion
  history.
- `M029` `DONE` `Done-current` - Echo Optics obstruction and admission results.
  Keep as completion history.
- `M030` `DONE` `Done-current` - Echo Optics open and close models. Keep as
  completion history.
- `M031` `DONE` `Done-current` - Echo Optics reading envelope and identity. Keep
  as completion history.
- `M032` `DONE` `Done-current` - Echo Optics stale-basis obstruction tests.
  Keep as completion history.
- `M033` `DONE` `Done-current` - Echo Optics witness basis and retained key.
  Keep as completion history.
- `M035` `DONE` `Done-current` - Observer plans and reading artifacts. Keep as
  completion history.
- `M045` `BLOCKED` `Current` - Contract-aware receipts and readings. Important,
  but blocked by contract proof and envelope work.
- `M047` `OPEN` `Current` - Parent drift and owned-footprint revalidation.
  Aligns with causal-basis honesty.
- `M069` `OPEN` `Current` - Reading envelope family boundary. Strong current
  pull.
- `M132` `OPEN` `Current-after-tightening` - Hashable view artifacts. Reframe
  around `ReadIdentity`, witness basis, aperture, and projection/reducer
  versions.
- `M135` `BLOCKED` `Current-after-tightening` - Provenance as query semantics.
  Keep if rewritten as observer-relative reading/provenance query semantics.
- `M171` `OPEN` `Current-after-tightening` - Reading envelope inspector. Pull
  only after envelope families are clear enough to inspect.

### Wesley And Contract Hosting

- `M008` `DONE` `Done-current` - Wesley protocol consumer cutover. Keep as
  completion history.
- `M016` `DONE` `Done-current` - Existing EINT, registry, and observation
  boundary inventory. Keep as current evidence.
- `M018` `OPEN` `Current` - Echo contract-hosting roadmap. Human triage already
  says keep.
- `M034` `DONE` `Done-current` - Echo Wesley Gen optic request builders. Keep as
  completion history.
- `M036` `DONE` `Done-current` - Registry provider wiring and host boundary
  decision. Keep as completion history.
- `M039` `OPEN` `Current` - Wesley compiled contract-hosting doctrine. Human
  triage already says keep.
- `M053` `BLOCKED` `Current` - Authenticated Wesley Intent admission posture.
  Keep behind toy proof and contract-aware receipts.
- `M064` `DONE` `Done-superseded` - Echo-Wesley Gen v2 JSON deserializer.
  Closed because Wesley is becoming the Rust library/artifact owner.
- `M067` `BLOCKED` `Current-after-tightening` - jedit text contract MVP. Keep
  only as an example contract fixture, not as Echo core ontology.
- `M074` `OPEN` `Review-before-pull` - Wesley go-public docs/CI. Confirm what
  Echo owns versus what Wesley owns.
- `M075` `OPEN` `Review-before-pull` - Migration backfill script generation.
  Likely Wesley-owned unless Echo needs a host-side migration proof.
- `M076` `BLOCKED` `Review-before-pull` - Migration switch-over and contract
  validation. Revalidate after `M075` ownership decision.
- `M077` `OPEN` `Review-before-pull` - GraphQL operation parser for QIR. Likely
  stale in Echo if Wesley owns GraphQL/QIR parsing.
- `M078` `BLOCKED` `Review-before-pull` - SQL query plan generation from QIR.
  Very likely Wesley-owned or out of scope for Echo core.
- `M079` `OPEN` `Current` - Wesley to Echo toy contract proof. Strong current
  pull.
- `M080` `OPEN` `Review-before-pull` - TypeScript type generation from Wesley
  IR. Check whether this still belongs in Echo after the Rust library pivot.
- `M081` `BLOCKED` `Review-before-pull` - Zod validators from Wesley IR. Same
  ownership concern as `M080`.
- `M082` `BLOCKED` `Review-before-pull` - CBOR bridge from TS types to WASM
  Rust. Keep only if it is a canonical adapter boundary, not causal ontology.
- `M105` `OPEN` `Review-before-pull` - Reconcile relocated Wesley Echo schemas.
  Likely useful as cleanup, but inspect current paths first.
- `M133` `BLOCKED` `Current-after-tightening` - Schema hash chain pinning.
  Align with artifact identity and read/receipt identity, not old JSON IR.
- `M134` `OPEN` `Review-before-pull` - SchemaDelta vocabulary. May be
  Wesley-owned; inspect before pulling.
- `M136` `OPEN` `Stale-close` - Shadow REALM investigation. This looks like an
  old future-direction card, not current Echo work.
- `M137` `OPEN` `Future-park` - Multi-language generator survey. Probably
  Wesley-owned and not current Echo execution.

### Continuum, Suffix Admission, Import, And Interchange

- `M005` `OPEN` `Current` - Echo and git-warp compatibility sanity check.
  Recently reframed; keep.
- `M006` `DONE` `Done-current` - Live holographic strands. Keep as completion
  history.
- `M040` `DONE` `Done-current` - Witnessed suffix admission shells. Keep as
  completion history.
- `M044` `OPEN` `Review-before-pull` - Compliance reporting as a TTD protocol
  extension. Check whether TTD is still the right host name/path.
- `M057` `OPEN` `Current` - Continuum proof family runtime cutover. Aligns with
  Echo as a peer Continuum runtime.
- `M062` `OPEN` `Current` - Echo / git-warp witnessed suffix sync. Strongly
  aligned with current doctrine if Echo remains peer, not subordinate.
- `M063` `OPEN` `Current-after-tightening` - Split `echo-session-proto` into
  retained bridge contracts vs legacy transport residue. Good cleanup target;
  avoid building a new host-bag abstraction.
- `M066` `OPEN` `Current` - Import outcome idempotence and loop law. Current
  suffix/import correctness work.
- `M102` `OPEN` `Review-before-pull` - git-mind NEXUS. Need evidence that this
  is still part of Echo's current integration map.
- `M103` `OPEN` `Review-before-pull` - Importer umbrella audit and close. Useful
  as an issue-closing pass if based on current import code.
- `M158` `BLOCKED` `Future-park` - Continuum contract artifact interchange.
  Coherent, but comes after contract hosting proof and artifact identity.

### Strands, Braids, Settlement, And Capability-Scoped Forking

- `M046` `BLOCKED` `Current` - Contract strands and counterfactuals. Aligns
  with generic strand/braid work, but depends on completed/active groundwork.
- `M051` `OPEN` `Merge-or-close` - Security/capabilities for fork/rewind/merge.
  Likely overlaps the Optics capability model and `M176`; consolidate before
  executing.
- `M065` `BLOCKED` `Stale-close` - Graft live frontier structural readings.
  Echo core must not gain Graft/editor-specific nouns. Rewrite generically or
  close.
- `M141` `OPEN` `Future-park` - Parallel execution counterfactuals. Conceptually
  aligned, but not a current substrate blocker.
- `M175` `OPEN` `Merge-or-close` - Merge semantics for admitted stream facts
  across worldlines. The useful part belongs in generic worldline/strand/braid
  admission and settlement semantics; do not pull as "stream facts."
- `M176` `OPEN` `Merge-or-close` - Security/capabilities for fork/rewind/merge
  in multiplayer. Consolidate with `M051` and the Optics capability model.

### Retention, CAS, Deep Storage, And Cached Readings

- `M052` `DONE` `Done-superseded` - TimeStream retention, spool compaction, and
  wormhole density. Closed as obsolete vocabulary.
- `M058` `BLOCKED` `Current` - Contract artifact retention in `echo-cas`.
  Strongly aligned with current retention doctrine.
- `M060` `OPEN` `Current` - MemoryTier WASM compilation gate. Good narrow
  current pull.
- `M061` `BLOCKED` `Current` - JS bindings for CAS store/retrieve. Current after
  `M060`.
- `M160` `OPEN` `Review-before-pull` - `Arc<[u8]>` to `bytes::Bytes`
  migration. Could be useful, but should be justified by measured storage/API
  friction.
- `M161` `BLOCKED` `Future-park` - `AsyncBlobStore` trait. Likely useful later;
  not needed until async storage is a real bottleneck.
- `M162` `BLOCKED` `Future-park` - Enumeration and metadata API. Keep behind a
  concrete retention/debugging need.
- `M163` `OPEN` `Current-after-tightening` - File-per-blob DiskTier
  implementation. Align with finite cache pressure, but keep CAS bytes separate
  from ontology.
- `M164` `BLOCKED` `Current-after-tightening` - Tiered promotion/demotion. Good
  after disk tier; must not affect causal identity.
- `M165` `BLOCKED` `Current-after-tightening` - Mark-sweep reachability
  analysis. Useful if it respects retained reading identity and witness needs.
- `M166` `BLOCKED` `Current-after-tightening` - Eviction policy and background
  sweep. Must fail closed with obstruction when evidence is missing.
- `M167` `OPEN` `Future-park` - Deep-storage wire protocol messages and binary
  encoding. Keep after local retention proves the boundary.
- `M168` `BLOCKED` `Future-park` - Deep-storage request/response protocol and
  backpressure. Too early unless networked retention becomes active.
- `M174` `DONE` `Done-superseded` - TimeStream retention and wormhole density.
  Same closure as `M052`; keep closed.

### Browser, WASM, TTD, And Visualization Hosts

- `M037` `OPEN` `Review-before-pull` - Commit-ordered rollback playbooks for
  TTD integration. Human triage already says this needs more information.
- `M038` `OPEN` `Current` - Reconcile TTD protocol schemas with `warp-ttd`.
  Human triage already says keep.
- `M054` `BLOCKED` `Current-after-tightening` - Canvas graph renderer. Update
  wording away from "static materialized reading" if it implies full hidden
  materialization.
- `M055` `BLOCKED` `Current-after-tightening` - Live tick playback and rewrite
  animation. Keep as an observation/replay surface, not mutable runtime truth.
- `M056` `BLOCKED` `Current` - Node inspection panel. Useful once the browser
  host bridge exists.
- `M070` `OPEN` `Current` - Narrow `ttd-browser` into an Echo browser host
  bridge. Current if it stays a bridge, not an ontology.
- `M071` `OPEN` `Current` - Wire Engine lifecycle behind `wasm-bindgen`
  exports. Good current pull.
- `M072` `BLOCKED` `Current` - Snapshot and `ViewOp` drain exports. Current
  after `M071`.
- `M073` `BLOCKED` `Current-after-tightening` - JS/WASM memory bridge and error
  protocol. Keep deterministic/canonical boundary constraints explicit.
- `M129` `OPEN` `Current` - Fuzzing the port. Useful TTD hardening if it targets
  current bridge APIs.
- `M131` `OPEN` `Future-park` - Causal visualizer. Coherent, but not current
  core work.

### Plugin, ABI, Sandbox, And Signing Security

- `M085` `OPEN` `Review-before-pull` - Rhai sandbox configuration. Confirm Rhai
  remains a live execution path before implementing.
- `M086` `BLOCKED` `Review-before-pull` - ViewClaim / EffectClaim receipts.
  Reframe through current receipt/reading doctrine if Rhai remains.
- `M088` `OPEN` `Review-before-pull` - C ABI spec. Old plugin ABI lane; confirm
  current plugin boundary before pulling.
- `M089` `BLOCKED` `Review-before-pull` - C header and host loader. Depends on
  the ABI ownership decision.
- `M090` `BLOCKED` `Review-before-pull` - Version negotiation. Depends on the
  ABI ownership decision.
- `M091` `BLOCKED` `Review-before-pull` - Capability tokens. Useful concept,
  but should align with Optics capability and admission posture.
- `M092` `BLOCKED` `Review-before-pull` - Example plugin and tests. Only after
  ABI direction is current.
- `M093` `OPEN` `Review-before-pull` - Commit/manifest signing spec. Probably
  useful, but old issue lineage should be checked.
- `M094` `OPEN` `Current-after-tightening` - Security contexts. Align with
  Optic capability, actor/cause, and admission law.
- `M095` `BLOCKED` `Review-before-pull` - FFI limits and validation. Depends on
  current FFI/plugin direction.
- `M096` `OPEN` `Review-before-pull` - JS-ABI packet checksum v2. Confirm this
  is still the active JS/WASM boundary.
- `M097` `OPEN` `Current-after-tightening` - Provenance payload v1. Keep aligned
  with receipts, witness basis, and causal identity.
- `M098` `OPEN` `Current` - ABI nested evidence strictness. Aligns with the
  current "evidence must be explicit" posture.
- `M106` `BLOCKED` `Review-before-pull` - Key management doc. Useful only after
  signing scope is current.
- `M107` `BLOCKED` `Future-park` - CI sign release artifacts dry run. Later
  release-hardening work.
- `M108` `BLOCKED` `Future-park` - CLI verify path for signatures. Later
  release-hardening work unless signing moves up.
- `M109` `BLOCKED` `Future-park` - CI verify signatures. Later
  release-hardening work.
- `M119` `OPEN` `Current-after-tightening` - Feature-gate contract
  verification. Current if the gate is about canonical contract boundaries, not
  hidden runtime behavior.

### Editor Hot Reload And Consumer-Specific Work

- `M099` `OPEN` `Stale-close` - Draft hot-reload spec. Do not make Echo core a
  file-handle/editor-hot-reload substrate; rewrite as adapter-only if needed.
- `M100` `BLOCKED` `Stale-close` - File watcher / debounce. Host-adapter
  concern, not Echo core.
- `M101` `BLOCKED` `Stale-close` - Hot-reload implementation. Host-adapter
  concern, not Echo core.
- `M112` `OPEN` `Future-park` - Reliving debugger UX design. Keep only as a
  consumer of optics/replay, not as a driver of core semantics.

### Time Travel, Admission Inspector, And Rulial Diff

- `M142` `BLOCKED` `Future-park` - Rulial diff / worldline compare MVP. Good
  idea after observation/read identities settle.
- `M143` `BLOCKED` `Future-park` - Wesley worldline diff. Depends on contract
  query/read proof work; not current frontier.
- `M144` `BLOCKED` `Future-park` - Provenance heatmap. Useful later as debug
  tooling.
- `M148` `BLOCKED` `Current-after-tightening` - Time travel core. Reframe around
  fixed ticks, playback coordinates, bounded reveal, and admitted timer history;
  avoid old stream/dt assumptions.
- `M149` `BLOCKED` `Future-park` - Reliving debugger MVP. Keep after `M148` and
  optics/replay APIs stabilize.
- `M173` `DONE` `Done-current` - Fixed timestep vs admitted dt stream. Closed;
  fixed timestep won.
- `M177` `BLOCKED` `Stale-close` - StreamsFrame inspector support. Rename or
  replace with generic admission/reading inspector; do not implement
  `StreamsFrame` as protocol truth.
- `M178` `BLOCKED` `Future-park` - Constraint Lens panel. The UX idea is fine,
  but it should consume typed admission explanations after the substrate exists.

### Example Apps, Game Fixtures, And Course Material

- `M139` `OPEN` `Future-park` - Splash Guy course material. Keep only if Splash
  Guy remains an active teaching fixture.
- `M140` `OPEN` `Future-park` - Tumble Tower course material. Keep only if
  Tumble Tower remains an active teaching fixture.
- `M145` `OPEN` `Future-park` - Splash Guy controlled desync. Example app lane,
  not current Echo substrate.
- `M146` `OPEN` `Future-park` - Splash Guy lockstep protocol. Example app lane,
  not current Echo substrate.
- `M147` `OPEN` `Future-park` - Splash Guy rules and state model. Example app
  lane, not current Echo substrate.
- `M150` `OPEN` `Future-park` - Tumble Tower desync breakers. Example app lane.
- `M151` `OPEN` `Future-park` - Tumble Tower lockstep harness. Example app lane.
- `M152` `OPEN` `Current` - Replay-from-checkpoint convergence tests. Generic
  enough to keep as core correctness work.
- `M153` `BLOCKED` `Current` - Replay-from-patches convergence property tests.
  Generic enough to keep after `M152`.
- `M154` `OPEN` `Future-park` - Tumble Tower stage 0 AABB. Example math lane.
- `M155` `OPEN` `Future-park` - Tumble Tower stage 1 rotation. Example math
  lane.
- `M156` `OPEN` `Future-park` - Tumble Tower stage 2 friction. Example math
  lane.
- `M157` `OPEN` `Future-park` - Tumble Tower stage 3 sleeping. Example math
  lane.
- `M172` `OPEN` `Future-park` - Splash Guy visualization. Example app lane.
- `M179` `OPEN` `Future-park` - Tumble Tower visualization. Example app lane.
