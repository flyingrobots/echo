<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0024 — Universal LE Binary Codec

_Replace per-boundary format diversity with a single deterministic little-endian
binary codec generated from Wesley IR across all serialization boundaries._

Legend: `PLATFORM`

Depends on:

- `0016 — Wesley-to-Echo toy contract proof` (established echo-wesley-gen pipeline)

---

## Why this cycle exists

Echo currently uses at least three serialization formats depending on context:

- **Canonical CBOR** (`cbor-canon-v1`) — generated vars payloads in
  `echo-wesley-gen`, encoded via `echo_wasm_abi::encode_cbor`.
- **Custom LE binary** (`echo-wasm-abi::codec`) — lower-level wire framing
  (EINT envelopes, TTD protocol). Already has `Writer`/`Reader` with LE
  primitives and length-prefixed strings.
- **JSON** — Wesley IR exchange, config, and diagnostics.

This is inconsistent. An app developer touching the WASM boundary, the WSC
format, and the network layer must understand three different formats and trust
that three separate implementations stay in sync.

The fix: use the existing `echo-wasm-abi::codec` LE binary format everywhere,
and have **Wesley generate all codec implementations simultaneously from the
same IR**. Because Wesley is the source of truth, drift between Rust, TypeScript,
WSC, and network representations is structurally impossible — not just unlikely.

---

## The core insight

The standard objection to raw binary formats is that they are not
self-describing: a version mismatch causes silent corruption instead of a
graceful skip. This objection assumes independent implementations that can
drift. Wesley eliminates that assumption.

```text
hot-text-runtime.graphql
  → echo-wesley-gen --rust         → Encode/Decode impls (codec.rs primitives)
  → echo-wesley-gen --typescript   → encode*/decode* functions (matching layout)
  → echo-wesley-gen --wsc          → WSC codec (future)
  → echo-wesley-gen --net          → network codec (future)
```

One schema compile. All representations emit atomically. Version gating is
handled by including the Wesley `SCHEMA_SHA256` as a prefix on every framed
message — version mismatch is a hard rejection, not silent corruption.

---

## Encoding table (ratified)

These mappings are canonical. Changing a mapping is a breaking change requiring
a new codec version.

| GraphQL type            | Rust type                | Wire encoding                                                                                                                                                                                                                             |
| ----------------------- | ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Int`                   | `i32`                    | 4 bytes, little-endian signed                                                                                                                                                                                                             |
| `Float`                 | `F32Scalar`              | Canonicalize via `F32Scalar::new()` first (flushes subnormals to `+0.0`, canonicalizes NaN to `0x7fc00000`, maps `-0.0` to `+0.0`), then 4 bytes LE bit pattern. TypeScript encoder must apply identical canonicalization before writing. |
| `Boolean`               | `bool`                   | 1 byte: `0x00` = false, `0x01` = true                                                                                                                                                                                                     |
| _(fixed-point alt)_     | `DFix64`                 | Q32.32 representation: `i64 LE` raw value (`real = raw / 2^32`). Used when a schema field is backed by `DFix64` instead of `Float`. Not a GraphQL scalar — a codec-level type.                                                            |
| `String`, `ID`          | `String`                 | `u32 LE` byte-length, then UTF-8 bytes (no null term)                                                                                                                                                                                     |
| `T!` (non-null scalar)  | `T`                      | value inline, no wrapper                                                                                                                                                                                                                  |
| `T` (nullable)          | `Option<T>`              | `u8` presence tag (`0x00` = null, `0x01` = present), then value if present                                                                                                                                                                |
| `[T!]!` (non-null list) | `Vec<T>`                 | `u32 LE` element count, then elements inline                                                                                                                                                                                              |
| `[T]` (nullable list)   | `Option<Vec<Option<T>>>` | presence tag, then count, then elements with per-element presence tags                                                                                                                                                                    |
| `enum E`                | `enum E`                 | `u32 LE` discriminant (variant index in declaration order)                                                                                                                                                                                |
| `type T` (object/input) | `struct T`               | fields encoded in declaration order, no separators                                                                                                                                                                                        |

**Alignment**: none. Fields are packed with no padding.

**String max bound**: enforced by the generator based on schema constraints
(default `usize::MAX` / `u32::MAX` bytes for unconstrained fields).

**Enum discriminant**: zero-based, ordered by SDL declaration order. Every
schema edit — append, insert, reorder, rename — is a breaking change against
peers running a different schema version, because the framing in Lines 206–214
hard-gates every payload by `SCHEMA_SHA256` and any of those edits perturbs
that hash. The declaration-order rule is therefore a determinism guarantee for
peers on the _same_ schema version, not a compatibility promise across them.

---

## What needs to be added to `codec.rs`

Current `codec.rs` has: `u8`, `u16`, `u32`, `i64`, length-prefixed bytes,
length-prefixed strings.

Missing primitives needed for full GraphQL type coverage:

- `write_i32_le` / `read_i32_le` — for `Int`
- `write_f32_le` / `read_f32_le` — for `Float`
- `write_bool` / `read_bool` — for `Boolean` (u8 0/1)
- `write_option` / `read_option` — presence tag + conditional payload
- `write_list` / `read_list` — u32 LE count + elements

These are all expressible with existing primitives today, but named helpers
make generated code readable and keep the TS mirror exact.

---

## `echo-wesley-gen` changes

### Existing (to be replaced)

```rust
pub fn encode_create_buffer_worldline_vars(
    vars: &CreateBufferWorldlineVars,
) -> Result<Vec<u8>, echo_wasm_abi::CanonError> {
    echo_wasm_abi::encode_cbor(vars)  // ← CBOR, going away
}
```

### Target (Rust emit)

```rust
impl echo_wasm_abi::codec::Encode for CreateBufferWorldlineVars {
    fn encode(&self, w: &mut echo_wasm_abi::codec::Writer) -> Result<(), echo_wasm_abi::codec::CodecError> {
        w.write_string(&self.buffer_key, usize::MAX)?;
        w.write_option(self.initial_text.as_deref(), |w, v| w.write_string(v, usize::MAX))?;
        w.write_option(self.projection_path.as_deref(), |w, v| w.write_string(v, usize::MAX))?;
        w.write_bool(self.create_initial_checkpoint.unwrap_or(false));
        Ok(())
    }
}

pub fn encode_create_buffer_worldline_vars(
    vars: &CreateBufferWorldlineVars,
) -> Result<Vec<u8>, echo_wasm_abi::codec::CodecError> {
    echo_wasm_abi::codec::encode_to_vec(vars)
}
```

### Target (TypeScript emit)

```typescript
// Generated by echo-wesley-gen. Do not edit.

export function encodeCreateBufferWorldlineVars(
    vars: CreateBufferWorldlineVars,
): Uint8Array {
    const w = new Writer();
    w.writeString(vars.bufferKey);
    w.writeOption(vars.initialText ?? null, (w, v) => w.writeString(v));
    w.writeOption(vars.projectionPath ?? null, (w, v) => w.writeString(v));
    w.writeBool(vars.createInitialCheckpoint ?? false);
    return w.finish();
}

export function decodeCreateBufferWorldlineResult(
    bytes: Uint8Array,
): CreateBufferWorldlineResult {
    const r = new Reader(bytes);
    return {
        worldline: decodeBufferWorldline(r),
        head: decodeRopeHead(r),
        checkpoint: r.readOption(() => decodeCheckpoint(r)),
    };
}
```

The TypeScript `Writer`/`Reader` is a ~100-line counterpart to `codec.rs`.

---

## TypeScript `codec.ts` primitive spec

The TypeScript reader/writer must mirror `codec.rs` exactly.

```typescript
class Writer {
    writeU8(v: number): void; // 1 byte
    writeU16Le(v: number): void; // 2 bytes LE
    writeU32Le(v: number): void; // 4 bytes LE
    writeI32Le(v: number): void; // 4 bytes LE signed
    writeF32Le(v: number): void; // canonicalize then 4 bytes LE: NaN→0x7fc00000, subnormal→0, -0→+0
    writeBool(v: boolean): void; // 1 byte (0x00 / 0x01)
    writeString(v: string): void; // u32 LE length + UTF-8
    writeOption<T>(v: T | null, fn: (w: Writer, v: T) => void): void;
    writeList<T>(vs: T[], fn: (w: Writer, v: T) => void): void;
    finish(): Uint8Array;
}

class Reader {
    readU8(): number;
    readU16Le(): number;
    readU32Le(): number;
    readI32Le(): number;
    readF32Le(): number;
    readBool(): boolean;
    readString(): string;
    readOption<T>(fn: (r: Reader) => T): T | null;
    readList<T>(fn: (r: Reader) => T): T[];
}
```

---

## Version / schema gating

Every framed message (WASM boundary, WSC, network) is prefixed with the 32-byte
`SCHEMA_SHA256` computed by Wesley at compile time. Decoders verify the hash
before reading the payload. Mismatch → hard rejection with an explicit error.

This is not per-field versioning. It is a hard schema version gate.
Upgrading schema requires regenerating both sides and redeploying together.
For WASM boundary and WSC, that is always already true. For network, it means
clients and servers must be at the same schema version, which is acceptable for
this system.

---

## Human users / jobs / hills

### Primary human users

- App developers wiring jedit to Echo.
- Platform engineers maintaining `echo-wesley-gen`.

### Human jobs

1. Run `echo-wesley-gen --schema hot-text-runtime.graphql --rust --out src/generated.rs`
2. Run `echo-wesley-gen --schema hot-text-runtime.graphql --typescript --out src/generated.ts`
3. Use the generated encode/decode functions in jedit and in Echo contract handlers.

### Human hill

A developer can serialize a jedit operation input in TypeScript and deserialize
it in Rust (or vice versa) without writing any codec code by hand.

---

## Implementation outline

1. **Extend `codec.rs`** — add `write_i32_le`, `write_f32_le`, `write_bool`,
   `write_option`, `write_list` and their `read_*` counterparts. Add tests for
   each primitive in both directions.

2. **Write `codec.ts`** — TypeScript mirror of `codec.rs`. Must be standalone
   (no runtime dependencies). Add roundtrip tests for each primitive.

3. **Add `--typescript` flag to `echo-wesley-gen`** — walking the same IR nodes
   as the Rust path, emit TypeScript encode/decode functions using `codec.ts`
   primitives. Field order must exactly match the Rust emit.

4. **Replace CBOR vars encoding** — update the Rust emit path in
   `echo-wesley-gen` to generate `Encode`/`Decode` impls instead of
   `encode_cbor` calls. Update generated code for existing contracts.

5. **Add roundtrip fixture tests** — for every operation in
   `hot-text-runtime.graphql`, encode a vars struct in Rust, decode it in
   TypeScript, encode it back in TypeScript, decode it in Rust. Assert identity.

---

## Tests to write first

- `codec.rs`: roundtrip for each new primitive (i32, f32, bool, option, list)
- `codec.ts`: roundtrip for each TypeScript primitive
- Cross-boundary: Rust-encoded bytes decode correctly in TypeScript (fixture vectors)
- Cross-boundary: TypeScript-encoded bytes decode correctly in Rust (fixture vectors)
- Generator: `echo-wesley-gen --typescript` emits functions that compile and
  roundtrip for the jedit hot-text fixture
- Schema hash gate: mismatched schema hash returns a hard error, not corruption

---

## Risks / unknowns

- **`f32` canonicalization**: The TypeScript `writeF32Le` must replicate `F32Scalar::new()`
  exactly: NaN → `0x7fc00000`, subnormal → `0`, `-0.0` → `+0.0`.
  JavaScript's `DataView.setFloat32` writes raw IEEE 754 bits without
  canonicalization. The pass must happen before the write.
  Roundtrip fixture tests must include NaN, subnormal, `-0.0`, and `+Infinity`
  as inputs to verify the TypeScript and Rust paths produce identical bytes.

- **`f64 → f32` narrowing in TypeScript**: JavaScript has no `f32` type.
  All numbers are `f64`. `DataView.setFloat32` silently narrows to the nearest
  representable `f32` per IEEE 754, which is deterministic, but any value that
  was computed in `f64` arithmetic may narrow differently than the same value
  computed in `f32`. Contract: **values passed to `writeF32Le` must already be
  exact `f32`-representable values**. The generator must emit this as a caller
  contract. Do not feed `f64` intermediate results into `writeF32Le`.

- **String Unicode normalization**: `TextEncoder` converts JavaScript's internal
  UTF-16 to UTF-8 without normalizing. The same logical string (e.g. `"café"`)
  has two valid UTF-8 byte sequences depending on NFC vs. NFD normalization,
  producing different digests. The codec encodes raw UTF-8 bytes as given — it
  does not normalize. Contract: **all strings must be NFC-normalized before
  encoding**. The application layer (not the codec) enforces this. This is
  especially critical for `insertText` in rewrite payloads where the content
  hash must be stable across platforms.

- **Field encoding order**: The TypeScript encoder emits fields in hardcoded
  generator-declared order, not by iterating JavaScript object keys. The
  generator must use Wesley IR declaration order for both Rust and TypeScript
  emit. If Wesley internally sorts fields for hashing purposes, that sorted
  order must never leak into the codec field order.
- **Large string / list bounds**: unconstrained fields default to `u32::MAX`.
  A malicious or buggy payload could allocate 4 GiB. The decoder should enforce
  a configurable max per decode context before reading.
- **CBOR migration**: existing generated code uses `encode_cbor`. jedit is the
  first product; there is no stored data in the old format. Hard cutover with no
  migration path needed.

---

## Postures

- **Accessibility:** Not applicable — this is a wire protocol.
- **Localization:** Not applicable.
- **Agent inspectability:** Generated code must be readable and have doc
  comments explaining the field order. An agent inspecting generated code
  must be able to reconstruct the wire layout from the source.

---

## Non-goals

- Replace EINT envelope framing (that already uses LE binary correctly).
- Design a self-describing format (Wesley is the description).
- Support schema evolution / field-skipping (hard version gate is sufficient).
- Generate WSC or network codec in this cycle (Rust + TypeScript first).
