<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Math Extraction to Bunny

- **Status:** Feasibility assessment; implementation not started
- **Date:** 2026-08-05
- **Echo basis:** `c354d531679861fb7bbd52ab7b7703807909ab86`
  (`origin/main` when this assessment began)
- **Bunny basis:** `9bf43600d08ff8e2a0ab888713948b409e386513`
  (`v0.6.0`, `main`, and `origin/main` in the inspected checkout)
- **Live work:** Any implementation status, dependency, or review state belongs
  in GitHub rather than in this document.

## Executable claim

Replacing Echo-owned reusable math and geometry with Bunny is feasible only if
the migration preserves Echo's committed bytes and causal posture while moving
pure numeric and geometric authority to Bunny.

The smallest decisive witnesses are:

1. every supported `f32`/raw-`i64` Q32.32 conversion produces identical raw
   bytes before and after the dependency change;
2. every migrated geometry operation either produces the same canonical Bunny
   value or maps Bunny's checked failure to an explicit Echo obstruction;
3. Echo receipts, payload identities, replay results, and WASM/native parity do
   not change unless a separately versioned protocol migration says they do;
4. `warp-core` retains no reusable math implementation after compatibility and
   protocol callers have moved to a Bunny-backed adapter.

## Executive finding

**The architectural replacement is desirable and already decided, but a full
replacement is not feasible against Bunny `v0.6.0`. A staged extraction is
feasible.**

[ADR 0019](../adr/0019-bunny-owns-reusable-geometry.md) already assigns reusable
scalar, vector, matrix, quaternion, transform, AABB, query, broad-phase, mesh,
and graphics-schema contracts to Bunny. It names `warp-math` and `warp-geom` as
staging/extraction surfaces rather than permanent Echo ontology. No new ADR is
required to execute that accepted boundary. A new or superseding ADR would be
required only if Echo intends to retain reusable math permanently, change the
canonical scalar away from signed Q32.32, or transfer causal authority to Bunny.

The current feasibility is split:

| Replacement target                                                         | Feasibility now                 | Finding                                                                                                                                                                                        |
| -------------------------------------------------------------------------- | ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Echo Q32.32 conversion helpers and `DFix64` kernel                         | **High after one prerequisite** | Bunny's raw representation, ties-to-even conversion, and saturating operators closely match Echo. The current Rust-version mismatch blocks a direct dependency today.                          |
| Echo fixed vectors and matrices                                            | **Moderate**                    | Bunny has the stronger canonical fixed API, but migration changes types, failure handling, and matrix storage. It is not a source-compatible substitution.                                     |
| `warp-geom::Aabb`                                                          | **Partial**                     | Bunny has validated `FixedAabb3`, but not Echo's complete float AABB utility and transform-aware bounds behavior.                                                                              |
| Echo `Vec3`, `Mat4`, `Quat`, trig, interpolation, and `Transform` behavior | **Low today**                   | Bunny `v0.6.0` intentionally leaves quaternion, angle/trig, interpolation, and transform-aware bounds for later work.                                                                          |
| Echo `Prng`                                                                | **No replacement identified**   | Bunny has seeded deterministic test corpora, but no public timeline-friendly PRNG library API equivalent to `warp_math::Prng`.                                                                 |
| Flag-day deletion of `warp-math` and `warp-geom`                           | **Not feasible**                | Public re-exports, payload code, geometry code, test lanes, policy scripts, and generated repository fixtures still refer to them. External callers are not visible in this source-only audit. |

Recommended disposition:

> Adopt Bunny bottom-up, beginning with a byte-identical Q32.32 adapter after
> resolving the Rust-version policy. Do not replace Echo's float-facing math or
> delete `warp-math`/`warp-geom` until Bunny lands the missing frame-math
> capabilities and compatibility witnesses prove each migrated surface.

## Why this follows Echo's architecture

Echo's durable territory is witnessed causal history. Pure arithmetic and
geometry do not gain causal authority merely because Echo invokes them. The
correct dependency shape is:

```text
witnessed Echo basis + bounded aperture + named law
                         |
                         v
                  Echo-owned adapter
                         |
                         v
              pure Bunny values/algorithms
                         |
                         v
        Echo obstruction, reading, or receipt binding
```

Bunny must not receive Echo worldlines, ticks, receipts, WAL handles, admission
authority, or ambient host capability. Echo translates admitted values into
Bunny's unitless numeric space, invokes a pure operation, and binds the result
or checked failure to the proposition Echo actually witnesses.

This is the boundary already stated by
[ADR 0019](https://github.com/flyingrobots/echo/blob/c354d531679861fb7bbd52ab7b7703807909ab86/docs/adr/0019-bunny-owns-reusable-geometry.md#L16-L32)
and the
[runtime constellation](https://github.com/flyingrobots/echo/blob/c354d531679861fb7bbd52ab7b7703807909ab86/docs/topics/RuntimeConstellation.md#L10-L61).

## Inspected surfaces

### Echo

The source inventory found three distinct numeric layers. They must not be
treated as one interchangeable library:

1. `warp-math` exposes the float-oriented compatibility surface: `Vec3`,
   `Mat4`, `Quat`, the `Scalar` trait, `F32Scalar`, feature-gated `DFix64`,
   deterministic trig, and `Prng`.
2. `warp-geom` builds float `Aabb`, quaternion-based `Transform`, and sampled
   `Timespan` bounds on that surface.
3. `warp-core` has a second minimal `Fx32`/`Vec3Fx` definition and publicly
   re-exports `warp_math::*` through `warp_core::math`.

The production references visible in this snapshot are narrow but important:

- `warp-core/src/payload.rs` uses the Q32.32 conversion helpers for canonical
  48-byte motion payloads;
- `warp-core/src/lib.rs` exposes the compatibility re-export;
- `warp-geom` consumes `Vec3`, `Mat4`, and `Quat`;
- `echo-dry-tests`, deterministic thread tests, and benchmarks exercise the
  float/fixed scalar lanes;
- policy scripts, local verification, determinism classification, docs, and
  checked-in `echo-wesley-gen` repository fixtures name the current crates;
- `echo-wasm-abi` has a separate float-to-fixed helper whose truncation policy
  does not match the ties-to-even payload path.

The local `Fx32` and `Vec3Fx` types have no references outside their defining
module in this snapshot. That makes them promising deletion candidates, but the
public module itself may have downstream callers and therefore still needs a
committed-caller or release-boundary audit.

### Bunny `v0.6.0`

Bunny provides:

- `bunny-num::FixedQ32_32`, raw `i64` access, saturating convenience operators,
  checked arithmetic, deterministic square root, checked float ingress, and
  saturating float ingress;
- `bunny-linalg::FixedVec2`, `FixedVec3`, fixed unit-vector proofs,
  `FixedMat2`/`3`/`4`, and `FixedAffine2`/`3`;
- `bunny-geom::FixedRay3`, `FixedAabb3`, and `FixedSphere3`, with checked float
  boundary conversions;
- deterministic query, broad-phase, mesh, codec, and contract crates beyond
  Echo's immediate math replacement need.

The current Q32.32 API is a substantially better canonical substrate than
Echo's raw conversion functions because it makes raw identity type-safe and
offers checked variants for operations where saturation would turn invalid
geometry into plausible data. See Bunny's
[`FixedQ32_32`](https://github.com/flyingrobots/bunny/blob/9bf43600d08ff8e2a0ab888713948b409e386513/crates/bunny-num/src/fixed_q32_32.rs#L19-L185)
and
[numeric constitution](https://github.com/flyingrobots/bunny/blob/9bf43600d08ff8e2a0ab888713948b409e386513/docs/NUMERIC_CONSTITUTION.md).

However, the repository explicitly records quaternion rotations, angle/trig,
interpolation, projection, and curves as open work. The `v0.6.0` bearing also
states that quaternion, angle, interpolation, curve, and transform-aware bounds
were moved to the next train. Those are present dependencies of Echo's current
float geometry surface, not optional polish.

## Compatibility matrix

| Echo surface                      | Bunny `v0.6.0` candidate                                                          | Compatibility judgment                    | Required treatment                                                                                                                                                               |
| --------------------------------- | --------------------------------------------------------------------------------- | ----------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `fixed_q32_32::{from_f32,to_f32}` | `bunny_num::fixed_q32_32::{from_f32,to_f32}`                                      | Near semantic match                       | Prove bit identity over a shared adversarial corpus before delegation. Preserve NaN-to-zero and infinity saturation where the existing payload contract requires them.           |
| `DFix64` raw type and arithmetic  | `FixedQ32_32`                                                                     | Strong semantic match                     | Replace internal use with the Bunny newtype. Preserve the `Scalar` facade temporarily only for compatibility callers.                                                            |
| `F32Scalar` canonicalization      | No equivalent canonical wrapper                                                   | Missing                                   | Keep only at explicit float compatibility boundaries or retire callers. Do not move float truth into Bunny merely to preserve the name.                                          |
| `Scalar` trait                    | No equivalent                                                                     | Missing by design                         | Prefer concrete Bunny fixed types. A permanent cross-backend trait would preserve Echo's dual numeric ontology and work against the extraction goal.                             |
| float `Vec3` arithmetic           | Bunny float `Vec3` is a boundary data type; `FixedVec3` owns canonical arithmetic | Not source compatible                     | Convert admitted float inputs through checked ingress, or preserve a local presentation-only adapter.                                                                            |
| float column-major `Mat4`         | row-major `FixedMat4` plus `FixedAffine3`                                         | Semantic overlap, representation mismatch | Use named constructors/adapters. Never reinterpret `[f32; 16]` or raw matrix storage. Prove point/vector and composition laws.                                                   |
| `Quat` and quaternion-to-matrix   | None                                                                              | Blocked                                   | Land the canonical quaternion/angle contract in Bunny first, including degeneracy and rotation-direction witnesses.                                                              |
| deterministic LUT `sin`/`cos`     | None                                                                              | Blocked                                   | Land Bunny's angle/trig policy first. Do not route canonical fixed math through float trig as an extraction shortcut.                                                            |
| `Prng`                            | None                                                                              | Blocked or retain outside math extraction | Determine whether any real runtime caller remains. If so, give deterministic randomness an explicit owner and identity/version contract rather than silently changing sequences. |
| `warp_geom::Aabb`                 | `FixedAabb3`                                                                      | Partial                                   | Basic validated bounds can move; union, padding, point construction, and transformed-bounds behavior need Bunny equivalents or narrow Echo adapters.                             |
| `warp_geom::Transform`            | `FixedAffine3`                                                                    | Partial                                   | Translation and linear transform overlap, but quaternion construction, non-uniform scale semantics, and float sentinels do not.                                                  |
| `warp_geom::Timespan::fat_aabb`   | None                                                                              | Blocked                                   | Move the explicit three-sample swept-bound law to Bunny with exact regression vectors before deleting Echo's implementation.                                                     |
| `warp_core::fixed::{Fx32,Vec3Fx}` | `FixedQ32_32`, `FixedVec3`                                                        | Likely replaceable                        | Confirm no external supported caller, then remove rather than create another compatibility alias.                                                                                |
| `warp_core::math::*`              | Bunny-backed compatibility facade                                                 | Transitional only                         | Preserve during migration; deprecate with a declared removal boundary after internal and known downstream callers move.                                                          |

## Non-equivalences that can change truth

### 1. Rust-version policy is an immediate blocker

Echo declares Rust `1.90.0`; Bunny `v0.6.0` declares Rust `1.96`. Cargo must
respect a dependency's `rust-version`, so Echo cannot honestly add the released
Bunny snapshot while continuing to claim the existing workspace toolchain.

Resolve this before code migration by choosing one of:

1. raise Echo's Rust version to 1.96 in its own focused, fully validated change;
2. establish and test a lower Bunny MSRV, then consume a Bunny release that
   declares it; or
3. wait for a release/toolchain alignment.

Do not copy Bunny source into Echo or use an undeclared toolchain exception.
Either would recreate the ownership split the migration is supposed to remove.

### 2. Matrix storage is different even where algebra agrees

Echo's float `Mat4` stores column-major arrays. Bunny's fixed matrices expose
row-major entries. Both use column-vector/right-to-left composition semantics,
so the mathematical convention is compatible while the byte and array layout
is not.

A migration adapter must construct Bunny rows by named element position and
must test:

- identity and transpose;
- translation affects points but not vectors;
- composition applies the inner transform first;
- every raw matrix element lands in the intended row and column;
- any external array or payload format remains unchanged.

The relevant Bunny contract is
[matrix and affine transform types](https://github.com/flyingrobots/bunny/blob/9bf43600d08ff8e2a0ab888713948b409e386513/docs/topics/matrix-types/README.md#L21-L72).

### 3. Failure semantics improve, and therefore differ

Echo's current float helpers often return sentinels:

- degenerate vector normalization returns zero;
- degenerate quaternion normalization returns identity;
- non-positive or non-finite square root returns zero;
- fixed scalar operators saturate, including division by zero.

Bunny's canonical geometry prefers validated constructors and checked
operations that return `None` or an error when input or intermediate arithmetic
is invalid. A direct wrapper that converts every failure back into zero,
identity, or saturation would discard the strongest reason to adopt Bunny.

Echo adapters should instead map checked failure to a named obstruction or
unavailable-evidence posture when the operation participates in admitted
history. Presentation-only callers may choose a sentinel, but that choice must
remain outside causal truth.

### 4. Unit ownership moves to the boundary

`warp-geom::Transform` describes translation in meters. Bunny's canonical
coordinates are deliberately unitless; downstream adapters assign meters,
pixels, tiles, or another scale. Echo must therefore bind the selected unit and
scale in the authored contract or adapter configuration rather than assuming
that the imported type carries meters.

See Bunny's
[coordinate law](https://github.com/flyingrobots/bunny/blob/9bf43600d08ff8e2a0ab888713948b409e386513/docs/topics/coordinate-law/README.md#L78-L131).

### 5. Wire conversion has two current Echo laws

`warp-core` motion payloads use ties-to-even Q32.32 conversion and write six
little-endian raw `i64` values. `echo-wasm-abi::fx_from_f32` instead scales in
`f64` and truncates toward zero. The canonical Echo docs already require this
mismatch to be resolved before a Bunny extraction is called parity-complete.

The migration must select one canonical implementation and retain golden
vectors in every language/ABI boundary. Existing payload bytes are history;
changing them requires a new payload version rather than a silent helper swap.

## Proposed migration

Each slice below has one executable claim and stops when its witness is green.
The slices are ordered; later slices must not absorb missing Bunny work into
Echo as an expedient.

### Slice 0: Align dependency policy

**Claim:** Echo can consume a released, immutable Bunny version under Echo's
declared Rust toolchain and license policy.

Actions:

- decide the Rust 1.90 versus 1.96 policy;
- verify the selected Bunny crates are available from the intended registry or
  use an explicitly pinned immutable source coordinate;
- audit license, lockfile, duplicate dependencies, native, MUSL, and WASM
  resolution;
- pin the initial migration to one reviewed Bunny release and record the exact
  source in the lockfile.

Witnesses:

- `cargo metadata --locked` under the declared Echo toolchain;
- `cargo tree` shows only the intended Bunny crates;
- native, MUSL, and `wasm32-unknown-unknown` compile checks resolve the same
  Bunny version.

Exit: dependency resolution is green without an undeclared toolchain override.

### Slice 1: Establish a cross-repository Q32.32 parity corpus

**Claim:** Bunny and the existing Echo implementation produce identical raw
Q32.32 conversion outputs for Echo's supported compatibility contract.

The corpus must include:

- `+0.0`, `-0.0`, all relevant subnormal boundaries, and ordinary normals;
- positive and negative exact-half ties with even and odd retained bits;
- the largest in-range values and adjacent out-of-range values;
- `i64::MIN`, `i64::MAX`, and raw values whose `f32` egress rounds upward;
- all NaN classes and both infinities for the legacy saturating path;
- deterministic seeded bit-pattern sampling;
- little-endian byte vectors consumed by Rust and the WASM/JavaScript boundary.

Place canonical numeric-law vectors with Bunny. Echo may retain only the
adapter/protocol vectors proving its payload contract.

Witness: the old Echo helper and Bunny implementation emit identical raw
`i64` values and egress `f32::to_bits()` values for the complete corpus.

Exit: no mismatch is unexplained; any intentional mismatch is versioned before
the dependency swap.

### Slice 2: Delegate Echo's Q32.32 implementation to `bunny-num`

**Claim:** canonical Echo payloads are byte-identical when produced through
Bunny.

Actions:

- change `warp-core` payload internals to construct and inspect
  `bunny_num::FixedQ32_32`;
- keep `warp_math::fixed_q32_32::{from_f32,to_f32}` as a thin, documented
  compatibility facade during the transition;
- use Bunny's checked ingress for new admitted geometry contracts;
- preserve the legacy saturating entry point only where the existing payload
  version promises NaN/infinity mapping;
- route the WASM parity helper through the same canonical law or give it a
  clearly different, versioned boundary name.

Witnesses:

- existing motion-payload golden bytes remain unchanged;
- decode/re-encode and legacy migration tests pass;
- Rust/native/MUSL/WASM parity corpus passes;
- no duplicate Q32.32 algorithm remains in production Echo code.

Exit: Bunny owns the numeric algorithm; Echo owns only protocol policy and
adapter naming.

### Slice 3: Remove unused duplicate fixed types

**Claim:** `warp_core::fixed::{Fx32,Vec3Fx}` is not a supported live contract.

Actions:

- scan exact-ref committed callers and known downstream workspaces;
- add a compile-fail or public-surface witness appropriate to the chosen
  compatibility posture;
- remove the duplicate types if no supported caller exists, or make the
  compatibility alias explicitly Bunny-backed and scheduled for removal.

Witness: the workspace and known downstream compatibility suite pass without
an independent Echo fixed representation.

Exit: one canonical fixed newtype remains.

### Slice 4: Migrate the Bunny capabilities that already exist

**Claim:** fixed vectors, matrices, affine transforms, and validated AABBs can
replace equivalent pure Echo algorithms without changing causal artifacts.

Actions:

- introduce a narrow Echo adapter that translates admitted values into
  `FixedVec3`, `FixedMat*`, `FixedAffine3`, and `FixedAabb3`;
- keep unit/space names in the adapter or authored contract;
- map checked overflow, singular transforms, invalid bounds, and invalid
  directions to explicit obstruction codes;
- port geometry tests to exact raw values rather than float tolerances;
- move generally reusable law tests upstream to Bunny.

Witnesses:

- raw fixed vectors prove coordinate handedness and matrix composition;
- point/vector translation and overflow cases prove the checked posture;
- Echo receipts bind the exact input basis, Bunny operation/profile version,
  output raw values, and obstruction posture;
- replay is invariant across worker count, native/MUSL, and WASM targets.

Exit: Echo no longer implements any pure capability that Bunny `v0.6.0`
already owns.

### Slice 5: Land the missing frame-math capabilities in Bunny

**Claim:** Bunny can express the remaining reusable behavior needed to retire
`warp-math` and `warp-geom`.

Required Bunny work:

- fixed quaternion representation, normalization, multiplication, and
  quaternion-to-matrix conversion;
- deterministic angle and trig law with raw golden vectors;
- interpolation/remap law, including quaternion interpolation and degeneracy;
- transform-aware bounds and the explicit swept-bound sampling policy;
- any still-live deterministic PRNG contract, if Bunny is selected as its
  durable owner.

Do not mechanically port Echo's current float sentinels. Define Bunny's
canonical checked contracts first, then write Echo compatibility adapters only
where a shipped Echo boundary requires them.

Witness: Bunny's own native/WASM tests and release gates prove the capability
before Echo consumes it.

Exit: Bunny has a released surface covering every live pure operation identified
by the Echo caller inventory.

### Slice 6: Retire the staging crates

**Claim:** deleting Echo's reusable implementations leaves only causal adapters
and does not break supported callers or protocol identity.

Actions:

- migrate internal imports from `warp_math`/`warp_geom` to Bunny or the narrow
  Echo adapter;
- deprecate and then remove `warp_core::math` at a declared compatibility
  boundary;
- move or delete tests according to ownership: Bunny law tests upstream, Echo
  payload/receipt/replay tests remain in Echo;
- update determinism policy, scripts, docs, workspace manifests, and lockfile;
- regenerate `echo-wesley-gen` repository fixtures through their owning
  generator rather than hand-editing generated source assets;
- delete `warp-math` and `warp-geom` only after source and committed-caller
  scans are empty.

Witnesses:

- `rg` finds no production implementation or stale current-state claim naming
  the retired crates, except intentional historical material;
- generator check mode reproduces all owned fixtures exactly;
- focused Bunny-adapter, payload, ABI, replay, and downstream compatibility
  suites pass;
- Echo's full local verification is green on all deterministic lanes.

Exit: reusable math and geometry are supplied by Bunny; Echo contains only the
causal adapters required by its own protocol.

## Validation gates

Implementation should not be called complete until all applicable gates below
are green:

| Gate                 | Required evidence                                                                                                                                    |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Numeric identity     | Exact raw-`i64` and little-endian byte corpus across old Echo, Bunny, WASM, and JavaScript boundaries                                                |
| Arithmetic posture   | Checked overflow/division/normalization failures map to named Echo obstructions; no successful all-zero or identity sentinel masquerades as proof    |
| Geometry law         | Handedness, inclusive AABB contact, canonical pair order, matrix composition, and swept-bound sampling vectors                                       |
| Causal identity      | Payload IDs, hashes, receipts, frontiers, and replay results remain unchanged unless explicitly versioned                                            |
| Portability          | Native, MUSL, `wasm32-unknown-unknown`, and headless WASM tests use one locked Bunny release                                                         |
| Public compatibility | Known internal and downstream `warp_math`, `warp_geom`, and `warp_core::math` callers are migrated or covered by an intentional compatibility facade |
| Repository ownership | Bunny contains pure law/algorithm tests; Echo contains adapters, payload bytes, obstruction binding, receipts, and replay tests                      |
| Generated artifacts  | Owning generator check mode reproduces repository fixtures; no generated bytes are hand-edited                                                       |
| Documentation        | ADR 0019 and current architecture remain accurate; stale claims that Echo owns reusable math are removed from their canonical owners                 |

## Stop conditions

Stop the migration slice and report the boundary rather than broadening scope
if any of the following occurs:

- adding Bunny requires an unapproved Echo MSRV change;
- a payload byte, hash, receipt, or replay result changes without an approved
  version migration;
- a needed operation exists only in Echo and its Bunny contract is not yet
  defined;
- a checked Bunny failure has no lawful Echo obstruction mapping;
- a generated repository fixture would need manual editing;
- downstream compatibility cannot be determined from available exact refs;
- the proposed adapter would give Bunny causal, filesystem, network, WAL, or
  ambient host authority.

## Scope and intentional non-actions

This assessment does not:

- add a Bunny dependency;
- change Echo's Rust version;
- change numeric, payload, ABI, geometry, or replay behavior;
- edit Bunny;
- create GitHub issues or assert live Bunny roadmap priority;
- commit, push, open a pull request, merge, or release anything.

The crates.io API was queried on 2026-08-05 and reported
[`bunny-num`](https://crates.io/crates/bunny-num/0.6.0),
[`bunny-linalg`](https://crates.io/crates/bunny-linalg/0.6.0), and
[`bunny-geom`](https://crates.io/crates/bunny-geom/0.6.0) `0.6.0` as published,
not yanked, and requiring Rust 1.96. Registry state is live dependency evidence,
not permanent architecture truth; verify it again when implementation begins.

## Source anchors

Echo:

- [ADR 0019: Bunny Owns Reusable Geometry](https://github.com/flyingrobots/echo/blob/c354d531679861fb7bbd52ab7b7703807909ab86/docs/adr/0019-bunny-owns-reusable-geometry.md)
- [Runtime constellation and Bunny numeric boundary](https://github.com/flyingrobots/echo/blob/c354d531679861fb7bbd52ab7b7703807909ab86/docs/topics/RuntimeConstellation.md#L10-L74)
- [`warp-math` public surface](https://github.com/flyingrobots/echo/blob/c354d531679861fb7bbd52ab7b7703807909ab86/crates/warp-math/src/lib.rs#L4-L69)
- [`warp-core::math` compatibility re-export](https://github.com/flyingrobots/echo/blob/c354d531679861fb7bbd52ab7b7703807909ab86/crates/warp-core/src/lib.rs#L28-L38)
- [canonical motion payload conversion](https://github.com/flyingrobots/echo/blob/c354d531679861fb7bbd52ab7b7703807909ab86/crates/warp-core/src/payload.rs#L35-L62)
- [separate truncating WASM helper](https://github.com/flyingrobots/echo/blob/c354d531679861fb7bbd52ab7b7703807909ab86/crates/echo-wasm-abi/src/codec.rs#L359-L389)
- [Echo Rust version](https://github.com/flyingrobots/echo/blob/c354d531679861fb7bbd52ab7b7703807909ab86/Cargo.toml#L32-L50)

Bunny:

- [Bunny `v0.6.0` workspace and Rust version](https://github.com/flyingrobots/bunny/blob/9bf43600d08ff8e2a0ab888713948b409e386513/Cargo.toml#L16-L22)
- [`FixedQ32_32` representation and checked API](https://github.com/flyingrobots/bunny/blob/9bf43600d08ff8e2a0ab888713948b409e386513/crates/bunny-num/src/fixed_q32_32.rs#L19-L185)
- [fixed linear-algebra exports](https://github.com/flyingrobots/bunny/blob/9bf43600d08ff8e2a0ab888713948b409e386513/crates/bunny-linalg/src/lib.rs#L21-L67)
- [math and geometry capability map](https://github.com/flyingrobots/bunny/blob/9bf43600d08ff8e2a0ab888713948b409e386513/docs/MATH_GEOMETRY_CAPABILITY_MAP.md#L125-L172)
- [matrix layout and checked composition contract](https://github.com/flyingrobots/bunny/blob/9bf43600d08ff8e2a0ab888713948b409e386513/docs/topics/matrix-types/README.md#L21-L72)
- [unit and transform convention](https://github.com/flyingrobots/bunny/blob/9bf43600d08ff8e2a0ab888713948b409e386513/docs/topics/coordinate-law/README.md#L78-L131)
- [`v0.6.0` missing-capability watchpoints](https://github.com/flyingrobots/bunny/blob/9bf43600d08ff8e2a0ab888713948b409e386513/docs/BEARING.md#L52-L86)
