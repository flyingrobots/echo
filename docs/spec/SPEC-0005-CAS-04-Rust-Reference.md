<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0005-CAS-04 — Rust Reference

## 7. Rust reference implementation (drop-in)

Match your existing `ttdr_v2.rs` style: magic, version, fixed header, decode returns (value, consumed).

### 7.1 `cas_v1.rs` (shared)

```rust
// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

use thiserror::Error;

pub type Hash32 = [u8; 32];

pub const CAS_WIRE_VERSION_V1: u16 = 1;

pub const MAX_WANT_HASHES: usize = 65_536;
pub const MAX_HAVE_HASHES: usize = 65_536;
pub const MAX_PROVIDE_ENTRIES: usize = 8_192;
pub const MAX_BLOB_LEN: usize = 16 * 1024 * 1024;

pub const MAX_FRAME_RAW_REFS: usize = 65_536;
pub const MAX_FRAME_TYPED_REFS: usize = 16_384;
pub const MAX_FRAME_ATTACHMENTS: usize = 16_384;

#[derive(Debug, Error)]
pub enum CasWireError {
    #[error("incomplete header: need {needed} bytes, got {got}")]
    IncompleteHeader { needed: usize, got: usize },

    #[error("bad magic: expected {expected:?}, got {got:?}")]
    BadMagic { expected: [u8; 4], got: [u8; 4] },

    #[error("unsupported version: expected {expected}, got {got}")]
    UnsupportedVersion { expected: u16, got: u16 },

    #[error("nonzero flags not supported in v1: {0}")]
    UnsupportedFlags(u16),

    #[error("count {count} exceeds max {max}")]
    CountTooLarge { count: usize, max: usize },

    #[error("not canonical: hashes not strictly increasing (unsorted or dup) at index {index}")]
    NotCanonicalSet { index: usize },

    #[error("incomplete payload: need {needed} bytes, got {got}")]
    IncompletePayload { needed: usize, got: usize },

    #[error("blob too large: {len} exceeds max {max}")]
    BlobTooLarge { len: usize, max: usize },

    #[error("provide entries not sorted by hash at index {index}")]
    ProvideNotSorted { index: usize },

    #[error("frame typed refs not sorted at index {index}")]
    TypedRefsNotSorted { index: usize },
}

// ----- basic LE readers -----

#[inline]
pub fn read_u16_le(bytes: &[u8], off: usize) -> Result<u16, CasWireError> {
    if bytes.len() < off + 2 {
        return Err(CasWireError::IncompletePayload { needed: off + 2, got: bytes.len() });
    }
    Ok(u16::from_le_bytes([bytes[off], bytes[off + 1]]))
}

#[inline]
pub fn read_u32_le(bytes: &[u8], off: usize) -> Result<u32, CasWireError> {
    if bytes.len() < off + 4 {
        return Err(CasWireError::IncompletePayload { needed: off + 4, got: bytes.len() });
    }
    Ok(u32::from_le_bytes([bytes[off], bytes[off + 1], bytes[off + 2], bytes[off + 3]]))
}

#[inline]
pub fn read_hash32(bytes: &[u8], off: usize) -> Result<Hash32, CasWireError> {
    if bytes.len() < off + 32 {
        return Err(CasWireError::IncompletePayload { needed: off + 32, got: bytes.len() });
    }
    Ok(bytes[off..off + 32].try_into().unwrap())
}

#[inline]
pub fn check_v1_header(bytes: &[u8], expected_magic: [u8; 4]) -> Result<(u16, u16), CasWireError> {
    // need 8 bytes
    if bytes.len() < 8 {
        return Err(CasWireError::IncompleteHeader { needed: 8, got: bytes.len() });
    }
    let magic: [u8; 4] = bytes[0..4].try_into().unwrap();
    if magic != expected_magic {
        return Err(CasWireError::BadMagic { expected: expected_magic, got: magic });
    }
    let version = read_u16_le(bytes, 4)?;
    if version != CAS_WIRE_VERSION_V1 {
        return Err(CasWireError::UnsupportedVersion { expected: CAS_WIRE_VERSION_V1, got: version });
    }
    let flags = read_u16_le(bytes, 6)?;
    if flags != 0 {
        return Err(CasWireError::UnsupportedFlags(flags));
    }
    Ok((version, flags))
}

#[inline]
pub fn ensure_canonical_set(sorted_hashes: &[Hash32]) -> Result<(), CasWireError> {
    // strictly increasing => sorted + deduped
    for i in 1..sorted_hashes.len() {
        if sorted_hashes[i - 1] >= sorted_hashes[i] {
            return Err(CasWireError::NotCanonicalSet { index: i });
        }
    }
    Ok(())
}
```

---

## 7.2 `cas_v1_want.rs`

```rust
use crate::cas_v1::*;

pub const WANT_MAGIC: [u8; 4] = *b"WANT";
pub const WANT_FIXED_HEADER_SIZE: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WantV1 {
    pub hashes: Vec<Hash32>,
}

pub fn encode_want_v1(hashes: &[Hash32]) -> Vec<u8> {
    let mut hs = hashes.to_vec();
    hs.sort();
    hs.dedup();

    let mut out = Vec::with_capacity(WANT_FIXED_HEADER_SIZE + 32 * hs.len());
    out.extend_from_slice(&WANT_MAGIC);
    out.extend_from_slice(&CAS_WIRE_VERSION_V1.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // flags
    out.extend_from_slice(&(hs.len() as u32).to_le_bytes());
    for h in hs {
        out.extend_from_slice(&h);
    }
    out
}

pub fn decode_want_v1(bytes: &[u8]) -> Result<(WantV1, usize), CasWireError> {
    check_v1_header(bytes, WANT_MAGIC)?;
    let count = read_u32_le(bytes, 8)? as usize;
    if count > MAX_WANT_HASHES {
        return Err(CasWireError::CountTooLarge { count, max: MAX_WANT_HASHES });
    }

    let needed = WANT_FIXED_HEADER_SIZE + 32 * count;
    if bytes.len() < needed {
        return Err(CasWireError::IncompletePayload { needed, got: bytes.len() });
    }

    let mut hashes = Vec::with_capacity(count);
    let mut off = 12;
    for _ in 0..count {
        hashes.push(read_hash32(bytes, off)?);
        off += 32;
    }

    // strict canonical
    ensure_canonical_set(&hashes)?;

    Ok((WantV1 { hashes }, needed))
}
```

---

## 7.3 `cas_v1_have.rs`

Same as `WANT` but magic "`HAVE`" and max `MAX_HAVE_HASHES`.

---

## 7.4 `cas_v1_provide.rs`

```rust
use crate::cas_v1::*;

pub const PROV_MAGIC: [u8; 4] = *b"PROV";
pub const PROV_FIXED_HEADER_SIZE: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvideEntryV1 {
    pub hash: Hash32,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvideV1 {
    pub entries: Vec<ProvideEntryV1>,
}

pub fn encode_provide_v1(entries: &[ProvideEntryV1]) -> Vec<u8> {
    let mut es = entries.to_vec();
    es.sort_by(|a, b| a.hash.cmp(&b.hash));

    // NOTE: We do NOT hash here; CAS store verifies later.

    let mut out = Vec::new();
    out.extend_from_slice(&PROV_MAGIC);
    out.extend_from_slice(&CAS_WIRE_VERSION_V1.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // flags
    out.extend_from_slice(&(es.len() as u32).to_le_bytes());

    for e in es {
        out.extend_from_slice(&e.hash);
        out.extend_from_slice(&(e.bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&e.bytes);
    }
    out
}

pub fn decode_provide_v1(bytes: &[u8]) -> Result<(ProvideV1, usize), CasWireError> {
    check_v1_header(bytes, PROV_MAGIC)?;
    let count = read_u32_le(bytes, 8)? as usize;
    if count > MAX_PROVIDE_ENTRIES {
        return Err(CasWireError::CountTooLarge { count, max: MAX_PROVIDE_ENTRIES });
    }

    let mut off = PROV_FIXED_HEADER_SIZE;
    let mut entries = Vec::with_capacity(count);

    let mut prev_hash: Option<Hash32> = None;

    for i in 0..count {
        let hash = read_hash32(bytes, off)?;
        off += 32;

        if let Some(prev) = prev_hash {
            if prev >= hash {
                return Err(CasWireError::ProvideNotSorted { index: i });
            }
        }
        prev_hash = Some(hash);

        let len = read_u32_le(bytes, off)? as usize;
        off += 4;

        if len > MAX_BLOB_LEN {
            return Err(CasWireError::BlobTooLarge { len, max: MAX_BLOB_LEN });
        }
        if bytes.len() < off + len {
            return Err(CasWireError::IncompletePayload { needed: off + len, got: bytes.len() });
        }

        let blob = bytes[off..off + len].to_vec();
        off += len;

        entries.push(ProvideEntryV1 { hash, bytes: blob });
    }

    Ok((ProvideV1 { entries }, off))
}
```

---

## 7.5 `cas_v1_frame.rs`

```rust
use crate::cas_v1::*;

pub const CFRM_MAGIC: [u8; 4] = *b"CFRM";
pub const CFRM_FIXED_HEADER_SIZE: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedRefV1 {
    pub schema_hash: Hash32,
    pub type_id: Hash32,
    pub layout_hash: Hash32,
    pub value_hash: Hash32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasFrameV1 {
    pub raw_refs: Vec<Hash32>,
    pub typed_refs: Vec<TypedRefV1>,
    pub attachments: Vec<Hash32>,
}

pub fn encode_cfrm_v1(frame: &CasFrameV1) -> Vec<u8> {
    let mut raw = frame.raw_refs.clone();
    raw.sort();
    raw.dedup();

    let mut att = frame.attachments.clone();
    att.sort();
    att.dedup();

    let mut typed = frame.typed_refs.clone();
    typed.sort_by(|a, b| {
        (a.schema_hash, a.type_id, a.layout_hash, a.value_hash)
            .cmp(&(b.schema_hash, b.type_id, b.layout_hash, b.value_hash))
    });

    let mut out = Vec::new();
    out.extend_from_slice(&CFRM_MAGIC);
    out.extend_from_slice(&CAS_WIRE_VERSION_V1.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // flags

    out.extend_from_slice(&(raw.len() as u32).to_le_bytes());
    out.extend_from_slice(&(typed.len() as u32).to_le_bytes());
    out.extend_from_slice(&(att.len() as u32).to_le_bytes());

    for h in raw { out.extend_from_slice(&h); }
    for tr in typed {
        out.extend_from_slice(&tr.schema_hash);
        out.extend_from_slice(&tr.type_id);
        out.extend_from_slice(&tr.layout_hash);
        out.extend_from_slice(&tr.value_hash);
    }
    for h in att { out.extend_from_slice(&h); }

    out
}

pub fn decode_cfrm_v1(bytes: &[u8]) -> Result<(CasFrameV1, usize), CasWireError> {
    check_v1_header(bytes, CFRM_MAGIC)?;

    let raw_count = read_u32_le(bytes, 8)? as usize;
    let typed_count = read_u32_le(bytes, 12)? as usize;
    let att_count = read_u32_le(bytes, 16)? as usize;

    if raw_count > MAX_FRAME_RAW_REFS {
        return Err(CasWireError::CountTooLarge { count: raw_count, max: MAX_FRAME_RAW_REFS });
    }
    if typed_count > MAX_FRAME_TYPED_REFS {
        return Err(CasWireError::CountTooLarge { count: typed_count, max: MAX_FRAME_TYPED_REFS });
    }
    if att_count > MAX_FRAME_ATTACHMENTS {
        return Err(CasWireError::CountTooLarge { count: att_count, max: MAX_FRAME_ATTACHMENTS });
    }

    let mut off = CFRM_FIXED_HEADER_SIZE;

    // raw refs
    let raw_needed = off + 32 * raw_count;
    if bytes.len() < raw_needed {
        return Err(CasWireError::IncompletePayload { needed: raw_needed, got: bytes.len() });
    }
    let mut raw_refs = Vec::with_capacity(raw_count);
    for _ in 0..raw_count {
        raw_refs.push(read_hash32(bytes, off)?);
        off += 32;
    }
    ensure_canonical_set(&raw_refs)?;

    // typed refs
    // each is 128 bytes
    let typed_bytes = 128 * typed_count;
    let typed_needed = off + typed_bytes;
    if bytes.len() < typed_needed {
        return Err(CasWireError::IncompletePayload { needed: typed_needed, got: bytes.len() });
    }

    let mut typed_refs = Vec::with_capacity(typed_count);
    let mut prev_tuple: Option<(Hash32, Hash32, Hash32, Hash32)> = None;

    for i in 0..typed_count {
        let schema_hash = read_hash32(bytes, off)?; off += 32;
        let type_id     = read_hash32(bytes, off)?; off += 32;
        let layout_hash = read_hash32(bytes, off)?; off += 32;
        let value_hash  = read_hash32(bytes, off)?; off += 32;

        let tup = (schema_hash, type_id, layout_hash, value_hash);
        if let Some(prev) = prev_tuple {
            if prev >= tup {
                return Err(CasWireError::TypedRefsNotSorted { index: i });
            }
        }
        prev_tuple = Some(tup);

        typed_refs.push(TypedRefV1 { schema_hash, type_id, layout_hash, value_hash });
    }

    // attachments
    let att_needed = off + 32 * att_count;
    if bytes.len() < att_needed {
        return Err(CasWireError::IncompletePayload { needed: att_needed, got: bytes.len() });
    }
    let mut attachments = Vec::with_capacity(att_count);
    for _ in 0..att_count {
        attachments.push(read_hash32(bytes, off)?);
        off += 32;
    }
    ensure_canonical_set(&attachments)?;

    Ok((CasFrameV1 { raw_refs, typed_refs, attachments }, off))
}
```

---

7.6 cas_v1_frame_plus.rs (CFRP)

use crate::cas*v1::*;
use crate::cas*v1_frame::*;
use crate::cas_v1_provide::\*;

pub const CFRP_MAGIC: [u8; 4] = \*b"CFRP";
pub const CFRP_FIXED_HEADER_SIZE: usize = 8;

# [derive(Debug, Clone, PartialEq, Eq)]

pub struct CasFramePlusV1 {
pub frame: CasFrameV1,
pub provide: ProvideV1,
}

pub fn encode_cfrp_v1(fp: &CasFramePlusV1) -> Vec<u8> {
let cfrm = encode_cfrm_v1(&fp.frame);
let prov = encode_provide_v1(&fp.provide.entries);

    let mut out = Vec::with_capacity(CFRP_FIXED_HEADER_SIZE + cfrm.len() + prov.len());
    out.extend_from_slice(&CFRP_MAGIC);
    out.extend_from_slice(&CAS_WIRE_VERSION_V1.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // flags
    out.extend_from_slice(&cfrm);
    out.extend_from_slice(&prov);
    out

}

pub fn decode_cfrp_v1(bytes: &[u8]) -> Result<(CasFramePlusV1, usize), CasWireError> {
check_v1_header(bytes, CFRP_MAGIC)?;
let mut off = CFRP_FIXED_HEADER_SIZE;

    let (frame, used1) = decode_cfrm_v1(&bytes[off..])?;
    off += used1;

    let (provide, used2) = decode_provide_v1(&bytes[off..])?;
    off += used2;

    Ok((CasFramePlusV1 { frame, provide }, off))

}

---

1. Storage boundary requirement (critical)

Where you store blobs (not in PROV decode), enforce:

- hash == blake3(bytes) else reject
- optionally: reject if already present (or allow idempotent)

This is what makes the whole system cryptographically honest.

---

1. Conformance test pack (complete)

9.1 Wire canonicalization tests

- WANT rejects unsorted hashes
- WANT rejects duplicates
- PROV rejects unsorted entries
- CFRM rejects unsorted raw_refs/attachments
- CFRM rejects unsorted typed_refs

    9.2 Bounds tests

- count > max → error
- blob len > max → error
- truncated payloads → IncompletePayload

    9.3 Bundle equivalence tests

- Decode CFRP, then verify:
- encode_cfrm_v1(frame) equals the CFRM slice bytes
- encode_provide_v1(entries) equals PROV slice bytes

    9.4 CAS correctness tests (store layer)

- Accept correct (hash, bytes)
- Reject incorrect (hash, bytes)
- Dedup behavior (optional)

    9.5 Worldline integration tests

Given a TTDR receipt (LIGHT/PROOF):

- client computes missing set {patch_digest, …}
- emits WANT
- receives PROV
- verifies patch bytes hash equals patch_digest
- (optional) replays patch to recompute state_root and match receipt

    9.6 Cross-platform determinism tests

Run on linux/mac/windows:

- same worldline tick produces identical patch_digest/state_root/commit_hash
- CAS makes drift impossible to hide.

---

1. “So where do we declare these?”

Answer: in Rust (echo-session-proto).
Treat them like TTDR: stable, minimal, bootstrap wire.

Wesley schema defines what the blobs mean, not how you ask for them.

---

Yep — you gave enough. Here’s the session envelope integration spec in full, wired to what you already have:
• WS framing: 1 WS binary frame = 1 JS-ABI packet = 1 OpEnvelope (unchanged).
• UDS framing: stream of JS-ABI packets, peeled by the JS-ABI LENGTH (unchanged).
• Hub is CAS-aware (your call) and becomes the dedupe/cache/quota brain.
• CAS messages are new Message variants carried inside the existing OpEnvelope session wire (so we don’t invent a new bootstrap tunnel).
• CAS blob bytes stay raw; we just carry them as bytes inside the CBOR payload.

This preserves your “session wire vs intent wire” boundary: TtdIntent remains an opaque EINT container, and the session layer still doesn’t parse EINT. ￼

⸻
