<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0005-CAS-01 — Foundations

> Ref-First Content Addressed Storage (CAS) v1

## 1. Axioms

1. All meaningful data is a Blob
2. Blob identity is its hash (`Hash32` = `BLAKE3(bytes)`)
3. Wire messages reference blobs by hash first
4. Inline bytes are optional, controlled, and never required for correctness
5. Any node can verify any claim locally (no trust in transport)

---

## 2. Core types

### 2.1 Hash

- **Hash32**: 32 bytes BLAKE3 digest
- **Canonical encoding**: 32 raw bytes (not hex)

### 2.2 Blob

- **Blob** = bytes
- `blob_hash` = `BLAKE3(Blob)`

### 2.3 BlobStore interface (normative)

Minimum operations:

- `has(hash) -> bool`
- `get(hash) -> bytes | NotFound`
- `put(bytes) -> hash` (_MUST_ verify returned hash equals `BLAKE3(bytes)`)
- `put_verified(hash, bytes)` (_MUST_ verify `BLAKE3(bytes) == hash` or reject)

Optional but useful:

- `pin(hash)` / `unpin(hash)`
- `gc(mark_roots)` / `reachable(root_hashes)`

---

## 3. Canonical encoding (Wesley’s job)

CAS is not a codec. It needs one canonical byte encoding for typed values. Wesley defines it.

**Rule**: a value is never hashed unless its bytes are canonical under the schema/layout.

Wesley outputs for each type layout:

- `encode(T) -> bytes (canonical)`
- `decode(bytes) -> T (strict)`
- `layout_hash` (hash of canonical schema layout)
- `type_id (major identity)`
- `value_hash(T) = BLAKE3(encode(T))`

**Float policy**: your deterministic float classes _MUST_ be the value representation used by `encode()`. Whatever invariants already enforced become the canonical bytes. (Perfect: it makes drift visible as hash divergence.)

---

## 4. Typed references (what goes on the wire)

### 4.1 `TypeId` and `LayoutHash`

- `type_id`: `Hash32` (stable for major; your choice how derived)
- `layout_hash`: `Hash32` (exact layout at minor; strict)

### 4.2 ValueRef (the universal pointer)

```rust
ValueRef {
  type_id: Hash32
  layout_hash: Hash32
  value_hash: Hash32   // hash of canonical bytes
}
```

**Rule**: Receivers _MUST_ treat `(type_id, layout_hash)` as required validation context for decoding.

### 4.3 `StringRef`

Stop doing "string tables" as sequential IDs. Do CAS strings.

```rust
StringRef { str_hash: Hash32 }
```

You can still batch-resolve them efficiently (see `Provide`), but identity is content.

---

## 5. Wire protocol (ref-first)

This is the minimal message set that actually works.

### 5.1 `WANT` (request missing blobs)

```rust
WANT {
  count: u32
  hashes: [Hash32; count]
}
```

**Rules**:

- _MUST_ dedupe internally
- _MAY_ rate-limit
- _MUST NOT_ request blobs already present

### 5.2 `HAVE` (optional hint)

```rust
HAVE {
  count: u32
  hashes: [Hash32; count]
}
```

Used to reduce chatty `WANT`s. Optional.

### 5.3 `PROVIDE` (send blobs)

```rust
PROVIDE {
  count: u32
  entries sorted by hash ascending:
    hash: Hash32
    len:  u32
    bytes: [u8; len]
}
```

**Rules**:

- Receiver _MUST_ verify `BLAKE3(bytes) == hash` or reject entry.
- Duplicate entries allowed if identical bytes; otherwise error.

### 5.4 `FRAME` (application message with refs)

A `FRAME` is always ref-first; it carries no required bytes.

```rust
FRAME {
  frame_type: Hash32     // identifies the "message schema" (TypeId or LayoutHash)
  payload_ref: ValueRef  // points at the typed payload blob
  attachments_count: u32
  attachments: [Hash32]  // optional extra blobs (shaders, textures, etc.)
}
```

If receiver lacks `payload_ref.value_hash`, it sends `WANT` for that hash.

### 5.5 `OPTIONAL`: `FRAME`+ (bundled for latency)

You can allow bundling to avoid RTT, while staying ref-first:

```rust
FRAME_PLUS {
  frame: FRAME
  provide: PROVIDE  // may include payload blob + any missing deps
}
```

**Rule**: `FRAME_PLUS` is an optimization only. The semantic meaning is identical to `FRAME` + `PROVIDE` sent separately.

---

## 6. Schema distribution (so decoding isn’t guesswork)

You need a way to ensure both sides agree on layouts.

### 6.1 `SchemaRef`

```rust
SchemaRef { schema_hash: Hash32 }
```

Schema itself is a blob in CAS:

- bytes = canonicalized GraphQL + directives (or Wesley IR)
- hash = `BLAKE3(bytes)`

### 6.2 Registry blob

Wesley produces a "registry" blob mapping:

- `type_id` -> known layout_hashes
- `layout_hash` -> decode metadata / generated module id
- plus any tooling metadata

This registry is also a CAS blob, referenced by hash.

Practical flow:

- session handshake includes `SchemaRef` or `RegistryRef`
- if missing, receiver `WANT`s it
- then receiver knows how to decode `layout_hash`

---

## 7. Validity rules (strict)

A node _MUST_ reject:

- any blob whose bytes don’t match its hash
- any `ValueRef` with unknown `layout_hash` unless "unknown layout allowed" mode is explicitly enabled (I’d default NO for Echo)
- any decode that is non-total (no panics, only typed errors)
- any attempt to interpret bytes under the wrong layout

This is where "we don’t fuck around" becomes enforceable physics.

---

## 8. Conformance tests

This is what makes ref-first real.

### 8.1 CAS core invariants

1. Hash correctness
    - For random bytes: `put(bytes)` returns hash equal to `BLAKE3(bytes)`.
    - `put_verified(hash, bytes)` rejects mismatches.
2. Idempotent store
    - Put same bytes twice -> same hash; store does not duplicate internally (optional but expected).

### 8.2 Wire invariants

1. Ref-first enforcement
    - `FRAME` must be processable without inline payload.
    - Missing payload triggers `WANT` exactly for missing hashes.
2. Provide verification
    - `PROVIDE` with wrong bytes for hash _MUST_ be rejected.
    - Partial `PROVIDE` _MUST_ store what verifies and reject what doesn’t.
3. Bundle equivalence
    - `FRAME_PLUS` _MUST_ be semantically identical to `FRAME` then `PROVIDE`.

### 8.3 Wesley codec invariants (cross-language)

1. Canonical encoding
    - `encode(T)` _MUST_ be byte-stable across runs.
    - `encode(decode(bytes))` _MUST_ produce the canonical form (idempotent).
2. Hash stability

- `value_hash(T)` _MUST_ match across Rust / TS / WASM for golden vectors

1. Schema/layout gating

- `ValueRef` with unknown `layout_hash` _MUST_ fail decode with `UnknownLayout`
- with known `layout_hash` + bytes present _MUST_ decode successfully

### D) Float determinism checks (the ones that matter)

1. Float class roundtrip

- `encode`->`decode` preserves your deterministic float representation exactly
- hash of encoded float vectors stable across platforms

1. Cross-platform determinism CI

- same golden vector suite on linux/windows/macos produces identical hashes
- (Optional) wasm runner produces identical hashes too

### E) Abuse prevention tests (your original "JSON pitfall" fear)

1. No dynamic types

- schema compiler rejects `JSON`/`Any`/`map`/`dict` constructs
- any attempt to encode "freeform" structures fails at compile time

---

## Practical "minimum viable ref-first" rollout

Do it in this order so you ship value fast:

1. `BlobStore` + `WANT`/`PROVIDE` (no schema yet, just blobs)
2. `SchemaRef` + Registry blob (CAS-distributed schema metadata)
3. `ValueRef` everywhere (`FRAME` references payload hash)
4. `FRAME_PLUS` (latency optimization)
5. `StringRef` (CAS strings; no string tables needed)
6. Pin+GC with worldline roots (production hygiene)

---

Ref-first CAS drops in beautifully on top of what we already have — because we’re already committing to hashes (`commit_hash`, `patch_digest`, `state_root`, `emissions_digest`). The only missing piece is: treat those hashes as first-class blob references, and add a tiny `WANT`/`PROVIDE` substrate.

Below is a strict "CAS-for-everything" spec that is compatible with your current TTDR v2 / receipt modes / retention model, and a test suite that will catch the nasty bugs.

---
