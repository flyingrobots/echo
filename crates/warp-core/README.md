<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# warp-core

Deterministic typed graph rewriting engine used by Echo.

This crate is the Rust core. See the repository root `README.md` for the full project vision and documentation index.

## What this crate does

- Implements the core deterministic engine used by Echo:
    - typed graph storage and snapshotting,
    - rule registration and application,
    - scheduler and drain logic,
    - commit hashing via BLAKE3.
- Provides the foundational APIs that `warp-wasm` and higher-level tools build
  on.

## Website kernel spike (WARP graphs)

The `warp-core` crate also contains a small “website kernel spike” used by the
`flyingrobots.dev` app:

- `WorldlineRuntime::ingest(IngressEnvelope)` is the live ingress surface:
    - envelopes resolve deterministically to a writer head by `DefaultWriter`,
      `InboxAddress`, or `ExactHead`,
    - per-head inboxes dedupe by content-addressed `ingress_id`,
    - committed duplicates are tracked per resolved writer head.
- `SchedulerCoordinator::super_tick(...)` is the internal logical cycle:
    - runnable writer heads advance in canonical `(worldline_id, head_id)` order,
    - commits run against the shared `WorldlineState` frontier for that worldline,
    - empty inboxes do not advance frontier ticks,
    - it is runtime internals, not a public WASM control export.
- `ObservationService::observe_optic(...)` carries the product read shape:
    - a request names an explicit causal coordinate, focus, bounded aperture,
      law versions, capability, and budget posture,
    - the current bridge validates supported focus/aperture/budget shape but
      does not verify capability or law identifiers against trusted authority,
    - generated `QueryBytes` optics currently obstruct, while raw
      `ObservationService::observe(...)` remains the installed-query and
      coordinate/frame/projection primitive.
- `WorldlineTick` and `GlobalTick` are explicit logical coordinates:
    - `WorldlineTick` is the monotone per-worldline append-order coordinate used
      to identify committed positions within one worldline,
    - `GlobalTick` is scheduler-cycle correlation metadata used to relate work
      across worldlines without implying wall-clock time or append order,
    - neither carries wall-clock semantics.
- Runtime control is intent-shaped:
    - domain writes and privileged scheduler/control requests both enter through
      canonical EINT intents,
    - scheduler lifecycle requests route through control intents and do not
      directly invoke `SchedulerCoordinator::super_tick(...)`.
- The runtime/kernel production path no longer uses `sim/inbox`,
  `edge:pending`, or `Engine::dispatch_next_intent(...)`.
- The current WASM boundary exposes `observe_optic(...)` for product reads;
  raw observation and neighborhood projections remain lower-level read-only
  surfaces, and scheduler metadata does not advance the runtime.
- `Engine::ingest_intent(intent_bytes)` and `Engine::ingest_inbox_event(seq, payload)`
  remain legacy compatibility helpers for isolated tests and older spike call sites.

## Generated provider package proposals

`warp-core` now accepts the provider-neutral registry emitted by the Edict
helper and can preflight one mutation into an opaque
`ProviderContractPackageProposalV1`. The constructor syntactically bounds the
host-owned runtime package occurrence to a nonempty name and version plus a raw
lowercase SHA-256 claim; it does not authenticate or semantically cross-bind
that occurrence. Separately, it fails closed unless the provider registry and
explicit host implementation agree on the complete operation id, Target IR,
semantic/release bundles, target/generated/operation profiles, provider and
value schemas, `le-binary-v1` codec, obstruction mapping, ABI, helper API, and
footprint identities, and unless the generated dispatch agrees on the exact
operation id and canonical rule name.

The generated helper owns the distinct typed input/output codecs and canonical
EINT construction; `warp-core` treats EINT `vars` as codec-owned opaque bytes.
The host supplies only its semantic mutation effects. Echo always adds the
generated matcher's mandatory ingress-EINT read to the proposed rule footprint
and conservatively enables every factor bit, so a host cannot omit those
Echo-owned matcher reads. Matching identities and footprints are preflight
claims, not proof that arbitrary host callback code implements the declared
semantics.

The proposal is deliberately non-installing. A trusted Echo host can now
compare its complete occurrence and provider-registry claims with an
independently constructed `ProviderContractAdmissionPolicyV1` and return an
opaque `AdmittedProviderContractPackageV1`. Semantic and release mismatch are
distinct typed failures, and neither success nor failure installs a handler or
invokes a callback. This first Echo crossing admits pinned claims; it does not
rehash the provider package bytes, register a rule, schedule work, emit a
receipt, or grant application authority. Exact package-byte corroboration and
provider-native installation remain later crossings. The mutation proposal
constructor still rejects query operations. Authored reads remain on the
separate bounded observer/optic path; they are never represented as synthetic
mutations.

## Documentation

- Core engine specs live in `docs/`:
    - `docs/spec/warp-core.md`, `docs/spec/scheduler-warp-core.md`,
      `docs/spec/canonical-inbox-sequencing.md`, `docs/spec/warp-tick-patch.md`,
      `docs/spec/merkle-commit.md`, and
      `docs/spec/SPEC-0004-worldlines-playback-truthbus.md`.
- Echo architecture and vocabulary live in `docs/architecture/outline.md` and
  `docs/theory/THEORY.md`.
