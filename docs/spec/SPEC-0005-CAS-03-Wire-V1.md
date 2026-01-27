<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0005-CAS-03 — Wire V1

## CAS wire v1 module layout

Put these next to `ttdr_v2.rs` in `echo-session-proto`:

```bash
echo-session-proto/src/
  cas_v1.rs            // mod + shared types/errors
  cas_v1_want.rs
  cas_v1_provide.rs
  cas_v1_frame.rs
```

Use type `Hash32 = [u8; 32];` (same shape as your engine-wide Hash). ￼

---

### 1) WANT v1 — request missing blobs

#### Wire format (Little-Endian)

```bash
offset size  field
0      4     magic = ASCII "WANT"
4      2     version = u16 LE (1)
6      2     flags = u16 LE (0 for now)
8      4     count = u32 LE
12     32*N  hashes = [Hash32; count]
```

#### Rust struct

```rust
// cas_v1_want.rs
pub type Hash32 = [u8; 32];

pub const WANT_MAGIC: [u8; 4] = *b"WANT";
pub const WANT_VERSION_V1: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WantV1 {
    pub flags: u16,
    pub hashes: Vec<Hash32>,
}

impl WantV1 {
    pub fn encode(&self, out: &mut Vec<u8>) -> Result<(), CasWireError> {
        // deterministic: hashes MUST be sorted + deduped
        let mut hashes = self.hashes.clone();
        hashes.sort();
        hashes.dedup();

        out.extend_from_slice(&WANT_MAGIC);
        out.extend_from_slice(&WANT_VERSION_V1.to_le_bytes());
        out.extend_from_slice(&self.flags.to_le_bytes());
        out.extend_from_slice(&(hashes.len() as u32).to_le_bytes());
        for h in &hashes {
            out.extend_from_slice(h);
        }
        Ok(())
    }

    pub fn decode(bytes: &[u8]) -> Result<(Self, usize), CasWireError> {
        // validate magic/version/bounds; return consumed bytes
        todo!()
    }
}
```

**Invariant**: `WANT` is a set (canonical sorted/deduped on encode). If someone spams duplicates, they don’t get extra bandwidth.

---

### 2) PROVIDE v1 — send blobs by hash

#### Wire format (Little-Endian)

```bash
offset size   field
0      4      magic = ASCII "PROV"
4      2      version = u16 LE (1)
6      2      flags = u16 LE (0 for now)
8      4      count = u32 LE

12     ...    entries sorted by hash asc:
              hash = [u8;32]
              len  = u32 LE
              bytes[len]
```

#### Rust structs

```rust
// cas_v1_provide.rs
pub type Hash32 = [u8; 32];

pub const PROV_MAGIC: [u8; 4] = *b"PROV";
pub const PROV_VERSION_V1: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvideEntryV1 {
    pub hash: Hash32,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvideV1 {
    pub flags: u16,
    pub entries: Vec<ProvideEntryV1>,
}

impl ProvideV1 {
    pub fn encode(&self, out: &mut Vec<u8>) -> Result<(), CasWireError> {
        // deterministic: entries MUST be sorted by hash
        let mut entries = self.entries.clone();
        entries.sort_by(|a, b| a.hash.cmp(&b.hash));

        out.extend_from_slice(&PROV_MAGIC);
        out.extend_from_slice(&PROV_VERSION_V1.to_le_bytes());
        out.extend_from_slice(&self.flags.to_le_bytes());
        out.extend_from_slice(&(entries.len() as u32).to_le_bytes());

        for e in &entries {
            out.extend_from_slice(&e.hash);
            out.extend_from_slice(&(e.bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(&e.bytes);
        }
        Ok(())
    }

    pub fn decode(bytes: &[u8]) -> Result<(Self, usize), CasWireError> {
        // MUST bounds-check len
        // receiver MUST verify blake3(bytes)==hash at storage boundary
        todo!()
    }
}
```

**Important**: `PROVIDE` does not compute hashes; it only transports. The storage layer enforces "bytes match hash" (CAS law).

---

### 3) CFRM v1 — ref-first application frame

This is the bridge between your existing receipts (TTDR) and CAS retrieval. It’s "here are the refs you might need," optionally typed.

#### Wire format (Little-Endian)

```bash
offset size  field
0      4     magic = ASCII "CFRM"
4      2     version = u16 LE (1)
6      2     flags = u16 LE

8      4     raw_ref_count   = u32 LE
12     4     typed_ref_count = u32 LE
16     4     attach_count    = u32 LE

20     32*N  raw_refs        = [Hash32; raw_ref_count]

...          typed_refs (typed_ref_count entries, fixed size):
              schema_hash  [32]   (ties to Wesley schema / registry)
              type_id      [32]
              layout_hash  [32]
              value_hash   [32]

...   32*M   attachments    = [Hash32; attach_count]
```

#### Rust structs

```rust
// cas_v1_frame.rs
pub type Hash32 = [u8; 32];

pub const CFRM_MAGIC: [u8; 4] = *b"CFRM";
pub const CFRM_VERSION_V1: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedRefV1 {
    pub schema_hash: Hash32,
    pub type_id: Hash32,
    pub layout_hash: Hash32,
    pub value_hash: Hash32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasFrameV1 {
    pub flags: u16,
    /// Un-typed hashes: patch_digest, schema_hash, registry_hash, etc.
    pub raw_refs: Vec<Hash32>,
    /// Typed refs: decode guarded by schema/layout identity.
    pub typed_refs: Vec<TypedRefV1>,
    /// Extra blobs (assets, scripts, etc.)
    pub attachments: Vec<Hash32>,
}

impl CasFrameV1 {
    pub fn encode(&self, out: &mut Vec<u8>) -> Result<(), CasWireError> {
        // deterministic: sort raw_refs and attachments; sort typed_refs lexicographically
        let mut raw_refs = self.raw_refs.clone();
        raw_refs.sort();
        raw_refs.dedup();

        let mut attachments = self.attachments.clone();
        attachments.sort();
        attachments.dedup();

        let mut typed_refs = self.typed_refs.clone();
        typed_refs.sort_by(|a, b| {
            (a.schema_hash, a.type_id, a.layout_hash, a.value_hash)
                .cmp(&(b.schema_hash, b.type_id, b.layout_hash, b.value_hash))
        });
        // NOTE: typed_refs can also be deduped if you want

        out.extend_from_slice(&CFRM_MAGIC);
        out.extend_from_slice(&CFRM_VERSION_V1.to_le_bytes());
        out.extend_from_slice(&self.flags.to_le_bytes());

        out.extend_from_slice(&(raw_refs.len() as u32).to_le_bytes());
        out.extend_from_slice(&(typed_refs.len() as u32).to_le_bytes());
        out.extend_from_slice(&(attachments.len() as u32).to_le_bytes());

        for h in &raw_refs {
            out.extend_from_slice(h);
        }
        for tr in &typed_refs {
            out.extend_from_slice(&tr.schema_hash);
            out.extend_from_slice(&tr.type_id);
            out.extend_from_slice(&tr.layout_hash);
            out.extend_from_slice(&tr.value_hash);
        }
        for h in &attachments {
            out.extend_from_slice(h);
        }
        Ok(())
    }

    pub fn decode(bytes: &[u8]) -> Result<(Self, usize), CasWireError> {
        todo!()
    }
}
```

#### What goes in raw_refs in Echo/Janus

This is where you glue into existing runtime concepts:

- `schema_hash` (already everywhere in your receipt hashing) ￼
- `patch_digest` (CAS key for `WorldlineTickPatchV1` blob) ￼
- `emissions_digest` or per-entry hashes if you decompose `TruthBus` values (you already hash entry bytes into the digest) ￼

Typed refs are for when you want "decode this exact thing under this exact schema/layout," not "here’s a blob."

---

### 4) Versioning policy (transport vs schema)

This is the "don’t forget to version stuff" part:

#### 4.1 CAS wire versioning (Rust, echo-session-proto)

- Each message has its own version: `u16` (like TTDR does) ￼
- Breaking change => bump that message’s version.
- Non-breaking => flags (`u16`) + optional trailing fields gated by flags (classic pattern).

#### 4.2 Typed payload versioning (Wesley)

- Schema identity: `schema_hash`
- Type identity: `type_id` (major)
- Layout identity: `layout_hash` (minor/exact)
- Value identity: `value_hash`

Transport stays stable; payload evolution rides through schema+layout.

---

## Final decision: Rust vs Wesley for these structs

- Do the wire structs in Rust (bootstrap layer).
- Do everything "semantic and typed" in Wesley (what those hashes refer to).

If you tried to put `WANT`/`PROVIDE` in GraphQL, you create a bootstrapping loop: you’d need the schema to fetch the schema. That’s... impressively unhelpful.

---

Alright James — here’s the complete, drop-in doc for Ref-First CAS wire v1, with Rust wire structs, versioning rules, canonicalization rules, decode/encode skeletons, and the test pack that makes it “physics, not vibes”.

This is designed to sit beside TTDR v2 in echo-session-proto (bootstrap layer), while Wesley/GraphQL owns typed payload schemas (the content you store/ship by hash).

---

SPEC-CAS-0001 — Echo Ref-First CAS Wire v1

1. Scope and layering

What is defined in Rust vs Wesley schema?

Rust (bootstrap wire):

- WANT, HAVE, PROV, CFRM, CFRP message structs and codecs
- These must exist before the peer has any Wesley registry/schema blobs.
- Location: echo-session-proto (same tier as ttdr_v2.rs)

Wesley/GraphQL (typed content):

- All domain payloads: tick patches, registry blobs, rule IR, game state objects, etc.
- Their canonical encoding is a Wesley concern (bytes that get hashed into CAS).

Reason: If you try to define WANT/PROV in GraphQL, you create a bootstrap loop:

“need schema to fetch schema to fetch schema”

No. Transport stays Rust.

---

1. Core primitives

1.1 Hash32

- Hash32 = [u8; 32] (BLAKE3 digest)
- Wire encoding: 32 raw bytes.

    1.2 Blob

- A blob is opaque bytes addressed by Hash32.
- CAS law: receiver must verify blake3(blob_bytes) == hash before accepting.

---

1. Canonicalization rules (the “WE DON’T FUCK AROUND” part)

All CAS wire messages have a canonical encoding so the bytes themselves can be hashed or logged deterministically.

2.1 Sets are canonical

For any Vec<Hash32> interpreted as a “set”:

- MUST be sorted ascending (lexicographic byte order)
- MUST be deduplicated
- Decoder in strict mode MUST reject out-of-order and duplicates

    2.2 PROVIDE entries are canonical

- Entries MUST be sorted by hash ascending.
- Duplicates are forbidden in strict mode.

    2.3 TypedRefs are canonical

- Sorted lexicographically by (schema_hash, type_id, layout_hash, value_hash).

---

1. Limits (DoS and sanity)

Defaults (tune as needed):

- MAX_WANT_HASHES = 65_536
- MAX_HAVE_HASHES = 65_536
- MAX_PROVIDE_ENTRIES = 8_192
- MAX*BLOB_LEN = 16* 1024 \_ 1024 (16 MiB)
- MAX_FRAME_RAW_REFS = 65_536
- MAX_FRAME_TYPED_REFS = 16_384
- MAX_FRAME_ATTACHMENTS = 16_384

Decoder MUST enforce these.

---

1. Message catalog (CAS Wire v1)

All are Little-Endian.

Common header pattern

offset size field
0 4 magic = ASCII
4 2 version = u16 LE (1)
6 2 flags = u16 LE

Flags are reserved for future extensions; v1 requires flags=0 unless specified.

---

4.1 WANT v1 — request missing blobs

Wire format

offset size field
0 4 magic = "WANT"
4 2 version = 1
6 2 flags
8 4 count = u32
12 32\*N hashes[count]

Semantics

- “Please send me blobs for these hashes if you have them.”
- Canonical: sorted + deduped.

---

4.2 HAVE v1 — hint availability (optional)

Wire format

offset size field
0 4 magic = "HAVE"
4 2 version = 1
6 2 flags
8 4 count = u32
12 32\*N hashes[count]

Semantics

- “I have these blobs” (to reduce pointless WANT spam).
- Canonical: sorted + deduped.

---

4.3 PROV v1 — provide blobs by hash

Wire format

offset size field
0 4 magic = "PROV"
4 2 version = 1
6 2 flags
8 4 count = u32
12 ... entries[count], sorted by hash asc:
hash[32]
len u32
bytes[len]

Semantics

- Transports blobs. Does not imply correctness.
- Receiver validates hash == blake3(bytes) before storing.

---

4.4 CFRM v1 — CAS Frame (ref-first application frame)

This is the “glue” message: it advertises refs needed for a higher-level semantic event (TTDR, EINT, Truth frames, etc.), and optionally typed refs.

Wire format

offset size field
0 4 magic = "CFRM"
4 2 version = 1
6 2 flags
8 4 raw_ref_count = u32
12 4 typed_ref_count = u32
16 4 attach_count = u32
20 32*N raw_refs[raw_ref_count]
... 128*M typed_refs[typed_ref_count]:
schema_hash[32]
type_id[32]
layout_hash[32]
value_hash[32]
... 32\*K attachments[attach_count]

Semantics

- raw_refs: untyped hashes (patch_digest, schema_hash, registry_hash, entry hashes, etc.)
- typed_refs: explicit typed-value references for decode-gated content.
- attachments: extra blobs (assets, scripts, etc.)

Canonicalization:

- raw_refs sorted/deduped
- typed_refs sorted
- attachments sorted/deduped

---

4.5 CFRP v1 — CAS Frame + Provide bundle (latency optimization)

Ref-first remains true: frame is primary; bytes are optional. CFRP just bundles bytes to avoid an RTT.

Wire format

offset size field
0 4 magic = "CFRP"
4 2 version = 1
6 2 flags

8 ... cfrm_bytes (must start with "CFRM", v1)
... ... prov_bytes (must start with "PROV", v1)

Parsing rule:

- Decode CFRM from offset 8; it returns (frame, consumed).
- Next bytes must decode as PROV.

---

1. Versioning policy (don’t screw this up)

5.1 CAS wire versioning (Rust)

- Each message has a version: u16.
- Breaking change => bump version.
- Non-breaking change => new flags + trailing sections only if length is known by counts.

    5.2 Typed payload versioning (Wesley)

A typed payload ref should include (at least):

- schema_hash
- layout_hash (exact minor)
- type_id (major)
- value_hash (CAS)

Decode is allowed only if the schema/layout is available (or you explicitly run in “unknown layout” mode, which Echo should default to NO).

---

1. Integration points (worldlines + TTDR)

This integrates cleanly with what you already have:

- TTDR.patch_digest becomes a CAS blob hash for WorldlineTickPatchV1 bytes.
- TTDR.state_root remains your Merkle root for state.
- TTDR.emissions_digest commits to hashes of emitted entry values — those can become CAS blobs too.

Practical flow

    1. Client receives TTDR in PROOF/LIGHT mode (hashes only).
    2. Client notices missing patch_digest blob.
    3. Client sends WANT { patch_digest }.
    4. Server replies with PROV { (patch_digest, patch_bytes) }.
    5. Client verifies and stores patch, then replays or inspects.

CFRM is a nicer general “ref advertisement” when you want to ship multiple refs and typed refs at once.

---
