<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0005-CAS-02 — Worldlines Integration

## SPEC-0005-CAS — Ref-First CAS for Echo Worldlines

### 1) Existing ground truth we build on

We already have:

- Receipt modes where `PROOF` is "hashes only, no bodies", and `LIGHT` is minimal hashes ￼
- `commit_hash:v2` that commits to `schema_hash`, `worldline_id`, `tick`, `parents`, `patch_digest`, optional `state_root`, and `emissions_digest`
- TTDR v2 layout carrying `schema_hash`, `worldline_id`, `tick`, `commit_hash`, `patch_digest`, `state_root`, `emissions_digest`, etc. ￼
- A clear retention/eviction model with rolling windows, LRU caches, fork eviction, etc. ￼

So: don’t invent a parallel system. Make CAS the storage/transport substrate for the hashes you already publish.

---

## 2) Canonical types

### 2.1 `Hash`

One type. One meaning. Everywhere.

- `Hash32 = [u8; 32]` (BLAKE3 digest)

(You already do this conceptually; TTDR v2 fields are `[u8;32]` everywhere. ￼)

### 2.2 `Blob`

- `Blob` bytes `B`
- `Blob` hash `H = BLAKE3(B)`
- Truth rule: a node accepts `(H, B)` only if `BLAKE3(B) == H`.

### 2.3 `TypedRef`

This is how you keep "WE DON’T FUCK AROUND" without CBOR-dumpster behavior:

```rust
TypedRef {
  schema_hash: Hash32      // which schema universe
  type_id: Hash32          // stable major identity (Wesley-generated)
  layout_hash: Hash32      // exact minor layout identity (Wesley-generated)
  value_hash: Hash32       // CAS hash of canonical value bytes
}
```

Decoding requires `(schema_hash, layout_hash)` to be known/loaded.

---

## 3) What becomes a blob

### 3.1 Mandatory blobs

1. Schema blob: canonicalized GraphQL schema bytes
    - `schema_hash` = `BLAKE3(schema_bytes)`
    - `commit_hash` already includes `schema_hash` ￼
2. Registry blob: Wesley output describing all types/layouts in that schema (for decoding and tooling)
    - registry format exists in v2 docs; keep the concept, but make it CAS-addressed. ￼
3. `WorldlineTickPatch` blob (per tick)
    - `patch_digest` in TTDR becomes the patch blob hash. TTDR already carries `patch_digest`. ￼
4. MBUS/Truth entries as blobs (optional but natural next step)
    - your `emissions_digest` is already computed from `hash32(entry_value_bytes)` ￼
    - make "`entry_value_bytes`" a canonical typed blob so it becomes shareable/provable.

### 3.2 Optional but recommended

- Checkpoint snapshot blobs for retention/eviction/restore (you already talk about checkpoint intervals and "snapshot-on-evict"). ￼
- String blobs (`StringHash = BLAKE3(utf8)`) to eliminate "string tables by arrival order."

---

## 1) Wire protocol (ref-first)

### 4.1 Messages

Minimal set:

#### `WANT`

- request missing blobs

```rust
WANT { count: u32, hashes: [Hash32] }
```

#### `PROVIDE`

- send blobs

```rust
PROVIDE {
  count: u32,
  entries sorted by hash asc:
    hash: Hash32
    len: u32
    bytes: [u8; len]
}
```

#### `FRAME`

- "here is a thing" (a receipt, an intent, a truth frame) referencing blobs

```rust
FRAME {
  // existing headers (session wire, etc.)
  refs: [Hash32]        // patch_digest, entry hashes, schema, registry, etc.
  typed_refs: [TypedRef] // when you want typed decoding guarantees
}
```

### 4.2 Where this sits

You already enforce "session wire vs intent wire" boundaries, with EINT passed through as bytes. ￼
CAS messages should live at the session wire layer (because they’re transport), while EINT/TTDR remain the payload semantics.

### 4.3 Bundling optimization (no semantic dependency)

Allow `FRAME_PLUS = FRAME + PROVIDE` in one packet to avoid RTT, but the system must function with pure ref-first.

---

## 5) How this maps onto Worldlines / TTDR

This is the key integration:

- TTDR v2 already carries:
    - `schema_hash` ￼
    - `worldline_id` ￼
    - `tick`
    - `commit_hash`
    - `patch_digest`
    - `state_root`
    - `emissions_digest` ￼

CAS interpretation:

- `patch_digest` is the CAS key for the tick patch blob.
- `state_root` is already a CAS-style Merkle root of state (treat as a root hash).
- `emissions_digest` commits to per-entry hashes already, which can become CAS references.

Receipt modes become CAS-native:

- `FULL`: includes bodies (or bundled `PROVIDE`)
- `PROOF`: includes only hashes (already a thing) ￼
- `LIGHT`: includes minimal hashes (commit/emissions/state) ￼

So you don’t redesign TTDR. You just give the client a way to fetch by hash.

---

## 6) Retention + GC roots (using your existing model)

Your retention section already defines windows/caches/checkpoints/fork eviction. ￼

CAS GC becomes simple:

### 6.1 Roots to pin

- Active worldline heads (by `WorldlineId`)
- Active cursors / forks (LRU eviction already described) ￼
- Latest N receipts (`receipt_cache_size`) ￼
- Checkpoint snapshots (every K ticks) ￼

### 6.2 Reachability rule

Pinned roots imply pinned blobs:

- Receipt pins: `schema_hash`, `patch_digest`, `emissions_digest`, optional channel digests
- Patch blob pins: whatever sub-blobs it references (if you decompose patches)
- Snapshot blob pins: state chunk blobs (if chunked)

Everything not reachable is evictable.

---

## Conformance / test plan (the "prove it" pack)

### A) Blob correctness

1. Reject wrong bytes

- `PROVIDE` contains (hash, bytes) with mismatch → reject entry, don’t store.

1. Idempotent store

- put same bytes twice → same hash, no duplicate storage.

### B) Ref-first behavior

1. `PROOF`/`LIGHT` must still work

- Receive TTDR in `LIGHT` mode → client _MUST_ be able to fetch missing patch bodies by `WANT`/`PROVIDE` and reconstruct needed views. (`LIGHT` definition is already minimal hashes) ￼

1. `WANT` dedupe

- Multiple frames reference same `patch_digest` → only one `WANT`.

1. Bundling equivalence

- `FRAME_PLUS` must be semantically equivalent to `FRAME` then `PROVIDE`.

### C) Worldline integrity

1. Patch digest integrity

- Retrieve patch blob by `patch_digest`, apply to parent state, recompute `state_root`, must match TTDR `state_root` when present ￼

1. Commit hash reproducibility

- Given the same inputs, recompute `commit_hash` exactly as per digest definition (`schema_hash`/`worldline_id`/`tick`/`parents`/`patch_digest`/`state_root`/`emissions_digest`) ￼

### D) Cross-platform determinism (CAS makes drift visible)

1. Same tick on linux/mac/windows:

- `patch_digest` identical
- `state_root` identical
- `commit_hash` identical

If it diverges, CAS doesn’t "break"; it catches the lie.

### E) Schema/registry gating

1. Unknown schema/layout rejection

- If client lacks `schema_hash` registry blob, it must `WANT` it before decoding typed blobs.
- If it cannot obtain it, it must fail with a deterministic error (`UnknownSchema`).

### F) Retention correctness

1. GC safety

- Pin active forks + last N receipts per your retention config ￼
- Run GC
- Assert pinned blobs remain; unpinned old blobs evicted.

---

## Migration plan that doesn’t blow up the repo

You don’t need a big-bang rewrite.

### Phase 1 — Patches first (lowest risk, highest value)

- Store `WorldlineTickPatchV1` bytes in CAS store keyed by `patch_digest` (TTDR already carries it) ￼
- Add `WANT`/`PROVIDE` to session wire
- Change "proof mode" client to fetch patch bodies on-demand

### Phase 2 — Schema/registry as blobs

- Make the Wesley registry a CAS blob and have sessions exchange `schema_hash` → `WANT` registry if missing
- This makes decoding portable and eliminates "hope both sides compiled the same build."

### Phase 3 — Truth entries as CAS

- Today `emissions_digest` commits to entry hashes ￼
- Make each entry value a CAS blob; keep `PROOF` mode tiny and fetch only what inspector opens.

---

## One blunt recommendation

Keep our existing TTDR v2 shapes. They’re already basically "CAS receipts." The line in our spec that says payload is canonical CBOR becomes:

> payload is either a hash ref or a bundled blob. (Your current EINT payload section is explicitly "canonical CBOR" right now---that’s the one you flip.)

---

The exact wire structs for `WANT`/`PROVIDE`/`FRAME` in the same "offset/size" style used for TTDR and the mapping of them onto our session wire message list (where they should live, how they’re multiplexed):

- CAS transport messages (`WANT`/`PROVIDE`/`FRAME`) should be Rust-defined in echo-session-proto (same tier as TTDR v2), because you need them to bootstrap before you necessarily have the Wesley schema/registry blobs. This matches your current approach where TTDR is a hand-written LE wire codec in Rust. ￼
- Typed payloads (patches, registry, rules, game state, etc.) should be Wesley/GraphQL-defined and CAS-addressed. Hashes and schema identity already exist everywhere (`Hash = [u8;32]` in `ident.rs`). ￼

Now: wire structs (v1), in the same "offset/size" style as TTDR.

---
