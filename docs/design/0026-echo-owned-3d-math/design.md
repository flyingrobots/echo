<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0026 — Echo-Owned 3D Math Primitives

_Make 3D math a generated, cross-language Echo contract surface: GraphQL SDL
defines the semantic primitives, `warp-math` supplies the deterministic runtime
authority, and Wesley emits Rust plus TypeScript types with one canonical raw
bytes codec._

Legend: `MATH`

Depends on:

- `0024 — Universal LE Binary Codec`
- `docs/determinism/SPEC_DETERMINISTIC_MATH.md`
- `crates/warp-math`
- `crates/warp-geom`

## Why this cycle exists

Echo already has deterministic math code in `warp-math` and geometry code in
`warp-geom`, but those types are not yet an authored contract surface that
external clients can consume without hand-written mirrors.

The current shape leaves three kinds of drift available:

- Rust runtime primitives can evolve without an equivalent generated
  TypeScript representation.
- TypeScript clients can treat vectors, quaternions, and matrices as ordinary
  numbers or object bags, bypassing Echo's deterministic float policy.
- Binary payload boundaries can preserve field values but fail to prove that
  the bytes passed through canonicalization, layout, and schema identity rules.

The fix is to make Echo-owned 3D math a generated contract family. GraphQL SDL
names the semantic primitives and their layout. Wesley compiles that SDL into
Rust types, TypeScript types, codec functions, schema hashes, and fixture
vectors. `warp-math` remains the executable authority for scalar
canonicalization and math operations; generated code routes all payload bytes
through that authority instead of reimplementing math policy at every boundary.

This gives application contracts a stable vocabulary for positions, directions,
rotations, transforms, bounds, and colors without making application nouns part
of Echo core.

## Human users / jobs / hills

### Primary human users

- Echo platform engineers maintaining deterministic math and codec boundaries.
- Application developers authoring Echo-hosted 3D, editor, or simulation
  contracts.
- Renderer/tooling maintainers consuming Echo values from TypeScript.

### Human jobs

1. Author a GraphQL contract that uses Echo 3D math primitives without writing
   per-language mirror types.
2. Inspect the exact wire layout and canonicalization rules for every primitive.
3. Compare Rust and TypeScript payloads with golden bytes when debugging
   cross-language behavior.
4. Extend the primitive family without weakening deterministic math policy.

### Human hill

A developer can add a contract field such as `Transform3D!` or `Vec3!` and get
matching Rust and TypeScript types plus canonical bytes without hand-authoring
codec glue.

## Agent users / jobs / hills

### Primary agent users

- Code-generation agents updating Wesley emitters.
- Test-writing agents producing fixture and roundtrip witnesses.
- Integration agents wiring jedit, renderer, or simulation consumers to Echo.

### Agent jobs

1. Generate Rust and TypeScript math types from one SDL authority.
2. Prove each primitive's canonical byte layout with fixture vectors.
3. Detect drift between runtime math types, generated schema metadata, and
   client payloads.
4. Report whether a math payload is canonical, rejected, or lossy at a boundary.

### Agent hill

An agent can read the SDL and generated fixtures and programmatically determine
the Rust type, TypeScript type, byte layout, canonicalization rule, and proof
command for every Echo-owned 3D math primitive.

## Ownership model

Echo owns the primitive contract. Applications own domain meaning.

| Layer                     | Owns                                                          | Does not own                   |
| ------------------------- | ------------------------------------------------------------- | ------------------------------ |
| GraphQL SDL               | primitive names, field order, semantic roles, schema identity | application-specific entities  |
| Wesley generator          | Rust/TypeScript types, codec functions, fixture metadata      | runtime math algorithms        |
| `warp-math`               | `F32Scalar`, `Vec3`, `Quat`, `Mat4`, deterministic operations | app contract nouns             |
| `warp-geom`               | `Aabb`, `Transform`, geometry helpers                         | renderer policy                |
| TypeScript client package | generated value shapes, encode/decode, validation helpers     | source-of-truth math semantics |
| Echo runtime              | admission, canonical bytes, receipts, retained evidence       | mutable renderer state         |

The authority order is:

1. `SPEC_DETERMINISTIC_MATH.md` and `warp-math` executable behavior.
2. Generated Rust/TypeScript golden byte fixtures.
3. GraphQL SDL schema hash and generated metadata.
4. Design docs.

GraphQL names the surface. It does not override deterministic math behavior.

## Contract surface

The Echo math SDL should live in a reusable contract module, for example:

```text
schemas/runtime/echo-math-3d.graphql
```

The initial surface should be intentionally small:

```graphql
scalar EchoF32

type Vec2 {
    x: EchoF32!
    y: EchoF32!
}

type Vec3 {
    x: EchoF32!
    y: EchoF32!
    z: EchoF32!
}

type Vec4 {
    x: EchoF32!
    y: EchoF32!
    z: EchoF32!
    w: EchoF32!
}

type Quat {
    x: EchoF32!
    y: EchoF32!
    z: EchoF32!
    w: EchoF32!
}

type Mat4 {
    columns: [EchoF32!]!
}

type Aabb {
    min: Vec3!
    max: Vec3!
}

type Transform3D {
    translation: Vec3!
    rotation: Quat!
    scale: Vec3!
}
```

### Naming posture

Use Echo names at the contract boundary:

- `EchoF32` for canonical float32 scalar values.
- `Vec3` for three-component spatial values.
- `Quat` for quaternion values stored as `(x, y, z, w)`.
- `Mat4` for column-major 4 by 4 matrix values.
- `Aabb` for world-space axis-aligned bounds.
- `Transform3D` for translation, rotation, and scale.

Do not use renderer names such as `THREE.Vector3` or application names such as
`BunnyPosition` in Echo-owned primitives. Renderer adapters may map generated
Echo values into renderer-specific types after decode.

### Role annotations

`Vec3` can represent a point, direction, normal, velocity, scale, or color.
Those meanings are not interchangeable even when their bytes match.

The SDL should support role annotations for application fields:

```graphql
directive @echo_math_role(
    role: EchoMathRole!
) on FIELD_DEFINITION | INPUT_FIELD_DEFINITION

enum EchoMathRole {
    POINT
    DIRECTION
    NORMAL
    VELOCITY
    SCALE
    RGB_LINEAR
    EXTENTS
}

input SpawnMeshInput {
    position: Vec3! @echo_math_role(role: POINT)
    forward: Vec3! @echo_math_role(role: DIRECTION)
    scale: Vec3! @echo_math_role(role: SCALE)
}
```

Role annotations are metadata for generated helpers, validation, docs, and
tooling. They do not change raw byte layout.

## Canonicalization contract

Every `EchoF32` entering an Echo-owned math payload must pass through the same
policy as `F32Scalar::new()`:

- `-0.0` maps to `+0.0`.
- All NaN payloads map to the canonical quiet NaN bit pattern `0x7fc0_0000`.
- Subnormal values flush to `+0.0`.
- Serialized bytes are the canonical little-endian `f32` bit pattern.

Composite values canonicalize component-by-component before hashing,
retention, comparison, and encode.

The first implementation may expose finite-only validators for common spatial
roles, but finite-only is a role policy, not the base `EchoF32` scalar policy.
Base scalar decode must be able to canonicalize any incoming `f32` bit pattern.

### Degenerate composite policy

Composite constructors should preserve `warp-math` behavior:

- `Vec3::normalize()` returns zero for magnitude at or below `EPSILON`.
- `Quat::normalize()` returns identity for degenerate quaternions.
- `Quat` raw decode does not silently normalize; generated helpers should
  expose explicit `normalized()` behavior when a field role requires a unit
  rotation.
- `Aabb` decode rejects `min > max` after component canonicalization.
- `Transform3D` decode preserves the supplied scale, including zero or negative
  scale, unless a field role or contract-specific validator forbids it.

## Raw bytes codec

This cycle extends `0024 — Universal LE Binary Codec`.

All Echo-owned math primitives use fixed-size packed little-endian layouts with
no padding and no alignment bytes.

| GraphQL primitive | Rust authority                        | TypeScript shape                   | Bytes | Layout                             |
| ----------------- | ------------------------------------- | ---------------------------------- | ----: | ---------------------------------- |
| `EchoF32`         | `F32Scalar`                           | `EchoF32` branded number           |     4 | canonical `f32` LE bits            |
| `Vec2`            | generated or future `warp-math::Vec2` | `{ x, y }`                         |     8 | `x`, `y`                           |
| `Vec3`            | `warp_math::Vec3`                     | `{ x, y, z }`                      |    12 | `x`, `y`, `z`                      |
| `Vec4`            | generated or future `warp-math::Vec4` | `{ x, y, z, w }`                   |    16 | `x`, `y`, `z`, `w`                 |
| `Quat`            | `warp_math::Quat`                     | `{ x, y, z, w }`                   |    16 | `x`, `y`, `z`, `w`                 |
| `Mat4`            | `warp_math::Mat4`                     | `readonly EchoF32[16]`             |    64 | column-major elements              |
| `Aabb`            | `warp_geom::Aabb`                     | `{ min, max }`                     |    24 | `min`, `max`                       |
| `Transform3D`     | `warp_geom::Transform`                | `{ translation, rotation, scale }` |    40 | `translation`, `rotation`, `scale` |

`Mat4.columns` must have exactly 16 elements. GraphQL list syntax is not enough
to express that bound, so Wesley must attach a generated fixed-array validator
for this type.

### Codec functions

Wesley should emit named primitive codecs in both languages:

```text
write_echo_f32 / read_echo_f32
write_vec3 / read_vec3
write_quat / read_quat
write_mat4 / read_mat4
write_transform3d / read_transform3d
```

Generated operation codecs should call those primitive codecs rather than
expanding component writes inline. That keeps primitive layout changes localized
and gives tests stable function names.

### Hashing and retention

Math payload hashes are hashes of canonical bytes, not hashes of source JSON,
debug strings, or renderer object identity.

For retained math artifacts:

- The semantic coordinate names the contract field, operation, or reading.
- The content hash names the canonical math bytes.
- Equal bytes under different semantic coordinates do not alias.
- Non-canonical incoming bytes are either canonicalized before retention or
  rejected with explicit validation evidence, depending on boundary posture.

## Generation model

The generator must consume one SDL authority and emit all language surfaces
atomically.

```text
echo-math-3d.graphql
  -> Wesley IR
    -> Rust types and Encode/Decode impls
    -> TypeScript types and encode/decode helpers
    -> schema hash and primitive registry metadata
    -> golden byte fixtures
```

### Rust emission

Rust generation should prefer existing runtime types where they already exist:

- `EchoF32` maps to `warp_math::scalar::F32Scalar`.
- `Vec3` maps to `warp_math::Vec3`.
- `Quat` maps to `warp_math::Quat`.
- `Mat4` maps to `warp_math::Mat4`.
- `Aabb` maps to `warp_geom::Aabb`.
- `Transform3D` maps to `warp_geom::Transform`.

Generated wrapper types are acceptable only when the existing runtime type does
not yet exist or when GraphQL nullability/role metadata needs a typed envelope.

Rust decode must construct values through validated constructors:

- `F32Scalar::new(bits_to_f32(...))`
- `Vec3::new(...)`
- `Quat::new(...)`
- `Mat4::new(...)`
- `Aabb::new(...)`
- `Transform::new(...)`

### TypeScript emission

TypeScript generation should not expose unbranded plain numbers for canonical
math values.

Target shape:

```typescript
export type EchoF32 = number & { readonly __echoF32: unique symbol };

export interface Vec3 {
    readonly x: EchoF32;
    readonly y: EchoF32;
    readonly z: EchoF32;
}

export interface Quat {
    readonly x: EchoF32;
    readonly y: EchoF32;
    readonly z: EchoF32;
    readonly w: EchoF32;
}
```

The TypeScript encoder must canonicalize before writing. The decoder must
canonicalize before returning branded values. JSON import helpers may exist for
developer ergonomics, but JSON is never the canonical math payload.

### Renderer adapters

Renderer adapters may provide convenience conversions:

```typescript
toThreeVector3(value: Vec3): THREE.Vector3
fromThreeVector3(value: THREE.Vector3): Vec3
```

Those conversions live in renderer/client packages, not in Echo core. They must
round-trip through `EchoF32` canonicalization before entering Echo payloads.

## Human playback

1. A developer writes `position: Vec3!` in an Echo-hosted contract.
2. Wesley generates Rust and TypeScript types plus `write_vec3` /
   `read_vec3`.
3. The developer encodes `{ x: -0.0, y: NaN, z: 1e-45 }` in TypeScript.
4. The emitted bytes are the same bytes Rust emits for the same logical value:
   `+0.0`, canonical NaN, `+0.0`.
5. The developer can compare the golden fixture instead of interpreting a
   renderer object dump.

## Agent playback

1. The agent reads `schemas/runtime/echo-math-3d.graphql`.
2. The agent runs the generator fixture command.
3. The output contains Rust types, TypeScript types, and golden bytes for each
   primitive.
4. The agent runs Rust and TypeScript roundtrip tests.
5. The agent determines whether every primitive preserved schema identity,
   canonical byte layout, and deterministic math policy.

## Implementation outline

1. Add `schemas/runtime/echo-math-3d.graphql` with the primitive SDL, role
   directive, and documentation comments.
2. Extend Wesley IR lowering so Echo math primitives become recognized
   generated scalar/composite types instead of ordinary application objects.
3. Add primitive codec helpers to Rust and TypeScript codec surfaces.
4. Emit Rust type mappings and Encode/Decode impls for existing `warp-math` and
   `warp-geom` types.
5. Emit TypeScript branded types, encode/decode helpers, canonicalization
   helpers, and renderer-neutral conversion helpers.
6. Add golden vectors for scalar hazards and composites.
7. Add retained-artifact fixture coverage proving math payloads hash canonical
   bytes and include schema identity.
8. Add docs index links only after the generated surface exists and is no
   longer design-only.

## Tests to write first

- Rust fixture: `EchoF32` canonicalizes `-0.0`, NaN payloads, and subnormals
  before LE encode.
- TypeScript fixture: `EchoF32` emits the same four bytes as Rust for the same
  hazard cases.
- Cross-language fixture: Rust encodes `Vec3`, TypeScript decodes and re-encodes
  identical bytes.
- Cross-language fixture: TypeScript encodes `Quat`, Rust decodes and
  re-encodes identical bytes.
- Layout fixture: `Mat4` preserves column-major order across both languages.
- Validation fixture: `Aabb` rejects `min > max` after canonicalization.
- Retention fixture: a math reading's content hash is the hash of canonical
  bytes, and semantic coordinates do not alias.
- Generator fixture: one SDL edit changes `SCHEMA_SHA256` and rejects stale
  payload frames before decode.

## Acceptance criteria

- [ ] Echo has a reusable `echo-math-3d.graphql` SDL module for 3D primitives.
- [ ] Wesley recognizes Echo math primitives as generated contract-owned
      primitives, not arbitrary application object bags.
- [ ] Rust generated code maps existing primitives to `warp-math` /
      `warp-geom` authority types.
- [ ] TypeScript generated code exposes branded, readonly math values and
      codec helpers.
- [ ] Raw bytes for every primitive are fixed-size, little-endian, packed, and
      fixture-proven.
- [ ] Scalar hazard cases have Rust and TypeScript golden vectors.
- [ ] Composite values canonicalize component-by-component before hashing or
      retention.
- [ ] Renderer adapters are optional clients of the generated values, not
      authorities over Echo math payloads.

## Validation plan

Initial docs-only validation:

- `npx --yes markdownlint-cli docs/design/0026-echo-owned-3d-math/design.md`
- `git diff --check`

Implementation validation, once slices exist:

- `cargo test -p warp-math`
- `cargo test -p warp-geom`
- `cargo test -p echo-wasm-abi`
- `cargo test -p echo-wesley-gen math_3d`
- `npm test --workspace packages/ttd-protocol-ts`
- cross-language golden fixture command emitted by the generator slice

## Risks / unknowns

- **GraphQL fixed arrays:** GraphQL list syntax cannot encode `Mat4[16]`
  statically. Wesley needs fixed-size generated validation for this primitive.
- **TypeScript NaN bits:** JavaScript numbers do not preserve arbitrary NaN
  payload identity. That is acceptable because Echo canonicalizes every NaN to
  one bit pattern, but tests must prove the writer emits `0x7fc0_0000`.
- **Existing `Vec3` constructors:** `warp_math::Vec3::new` currently accepts
  `f32` components directly. Generated decode must route each component through
  scalar canonicalization before constructing composites.
- **Role validation scope:** Normal, direction, and unit-quaternion validation
  can become expensive or policy-heavy. The first slice should keep base
  primitives canonical and make role validation explicit opt-in.
- **Renderer pressure:** Three.js and similar renderers prefer mutable objects.
  Echo-generated TypeScript values should remain immutable; renderer adapters
  can allocate mutable renderer values outside Echo payload authority.

## Postures

- **Accessibility:** Not directly applicable. This is a data-contract and codec
  design. Downstream visual tools should expose accessible descriptions of
  generated geometry readings separately.
- **Localization:** Not directly applicable. Primitive names and codec metadata
  are protocol surfaces, not user-facing copy.
- **Agent inspectability:** Required. SDL, generated metadata, fixture bytes,
  and validation commands must let an agent prove exact layout and
  canonicalization without reading renderer code.
- **Determinism:** Central. The design is invalid unless Rust and TypeScript
  emit identical canonical bytes for scalar hazards and composites.
- **Boundary honesty:** Required. Echo owns canonical payload bytes and runtime
  math authority; renderers and applications own only domain interpretation and
  presentation.

## Non-goals

- Replacing `warp-math` with generated math algorithms.
- Adding physics, collision solving, scene graphs, or renderer state to Echo
  core.
- Making JSON a canonical math payload.
- Making GraphQL SDL override `SPEC_DETERMINISTIC_MATH.md`.
- Normalizing every quaternion or vector on raw decode.
- Adding app-specific mesh, editor, or bunny-scene nouns to Echo-owned math
  primitives.
