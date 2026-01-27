<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0005-CAS-05 — Session Envelope Integration

## SPEC-CAS-0002 — CAS Session Envelope Integration (JIT!/OpEnvelope)

1. Current session envelope (baseline)

You already have:

1.1 JS-ABI packet (outer framing)

Packet layout is fixed: header(12) + payload + checksum(32). (You described it; your wire.rs enforces it.)

1.2 Session payload (inside JS-ABI)

Payload is a canonical CBOR OpEnvelope<P>, where:
• op: String dispatches message kind
• ts: u64 logical timestamp
• payload: P message-specific body

And you currently decode into Message variants like Handshake, SubscribeWarp, WarpStream, Notification. ￼

1.3 “Hard rule” boundary

Session wire is session control; intent wire is opaque EINT v2 inside Message::TtdIntent (you already designed this separation). ￼

⸻

1. Goal

Introduce ref-first CAS at the session wire layer so the hub can:
• dedupe WANTs across clients
• satisfy from local cache
• enforce blob size / provide limits
• backpressure or error deterministically

without changing WS framing or JS-ABI framing.

⸻

1. Add CAS message variants to Message (lib.rs)

3.1 New Message variants

Add these to your existing pub enum Message (the one that currently contains Handshake, HandshakeAck, Error, SubscribeWarp, WarpStream, Notification):

/// CAS: request missing blobs (WANT v1 bytes).
CasWant { bytes: Vec<u8> },

/// CAS: hint availability (HAVE v1 bytes).
CasHave { bytes: Vec<u8> },

/// CAS: provide blobs (PROV v1 bytes).
CasProvide { bytes: Vec<u8> },

/// CAS: advertise refs (CFRM v1 bytes).
CasFrame { bytes: Vec<u8> },

/// CAS: frame + provide bundle (CFRP v1 bytes).
CasFramePlus { bytes: Vec<u8> },

Important: the hub will parse bytes using your Rust CAS wire structs (WANT/PROV/CFRM/CFRP) — not CBOR — so CAS stays “raw + canonical + verifiable”.

3.2 Op names (stable)

Extend your Message::op_name() mapping with:

Variant op string
CasWant "cas_want"
CasHave "cas_have"
CasProvide "cas_provide"
CasFrame "cas_frame"
CasFramePlus "cas_frame_plus"

These are simple and versionless because the CAS bytes include their own magic + version (e.g., "WANT" + u16 version).

⸻

1. OpEnvelope payload shapes for CAS ops

Because your session wire payload is CBOR, we need a minimal CBOR representation that doesn’t “reintroduce JSON.”

4.1 Canonical CBOR payload for CAS ops

For each CAS op, the CBOR payload MUST be a struct with a single field:

# [derive(Serialize, Deserialize)]

pub struct CasBytesPayload {
pub bytes: Vec<u8>,
}

That’s it. No maps. No freeform.

Why: it prevents “oh I’ll just stash JSON in a map here” while still letting the session wire stay CBOR.

4.2 Encode path (encode_message)

Extend your existing encode_message match (which currently handles Handshake, SubscribeWarp, etc.) to include:
• Message::CasWant { bytes } => ("cas_want", encode_payload(&CasBytesPayload { bytes: bytes.clone() })?)
• …same for others.

4.3 Decode path (decode_message)

Extend your existing decode_message dispatch (which switches on env.op.as_str()) similarly.

⸻

1. Capability negotiation (Handshake)

Your handshake payload already has capabilities: Vec<String> and the hub echoes them back. Perfect.

5.1 Capability strings (v1)

Client includes:
• cas:ref-first:v1
• cas:frame-plus:v1 (optional; only if you implement bundling)
• cas:have:v1 (optional; if you want HAVE)

Hub returns the subset it actually enables.

5.2 Limits (capabilities OR session_meta)

You proposed putting limits into capabilities. That’s fine, but parsing strings is annoying. You already have session_meta: Option<BTreeMap<String, ciborium::Value>>, so use it.

Client → server session_meta (optional):
• cas.max_blob (u32)
• cas.max_provide_entries (u32)
• cas.max_want_hashes (u32)

Server → client session_meta (authoritative):
• same keys, final values

If you want the “string-only” style anyway, also allow:
• cas:max_blob=8388608
• cas:max_entries=64

…but the meta map is cleaner.

⸻

1. Hub semantics (CAS-aware hub)

This is the behavior contract for the hub loop that currently only understands the existing Message variants.

6.1 Hub responsibilities

When hub receives:

A) CasWant 1. Parse bytes as WANT v1 (strict canonical) 2. Enforce:
• count <= negotiated cas.max_want_hashes (or defaults) 3. Dedupe across sessions:
• maintain global pending_wants: HashSet<Hash32>
• per-session session_outstanding: HashSet<Hash32> to avoid repeated asks 4. For each hash:
• If blob in cache/store: enqueue it for CasProvide back to requester
• Else: mark as pending (and optionally forward upstream if hub has upstream sources)

B) CasProvide 1. Parse PROV v1 (strict canonical ordering) 2. Enforce:
• entry_count <= cas.max_provide_entries
• each blob len <= cas.max_blob and also <= WS gateway max_frame_bytes 3. Verify each entry:
• BLAKE3(bytes) == hash (reject entry if mismatch) 4. Store verified blobs in hub blob store 5. Wake any sessions waiting on those hashes and reply with CasProvide (or a CasFramePlus if you want latency wins)

C) CasFrame / CasFramePlus
• CasFrame is advisory: it advertises refs; hub can prefetch or serve from cache.
• CasFramePlus is CFRM + PROV in one payload:
• decode and process as “frame then provide,” same semantics.

6.2 Error handling (session errors, not domain errors)

You already have a “errors as truth” philosophy — session errors are just protocol-level issues. ￼

So for CAS protocol violations, hub sends Message::Error(ErrorPayload) with stable codes:

Code Name When
400 E_CAS_BAD_WIRE CAS bytes don’t parse / bad magic/version
409 E_CAS_NON_CANONICAL unsorted/dedup violation
413 E_CAS_PAYLOAD_TOO_LARGE blob > max_blob or message > gateway max
429 E_CAS_RATE_LIMIT quota/backpressure
500 E_CAS_INTERNAL hub failure

Rule: Hub MAY close the session on repeated 400/409.

⸻

1. Size and backpressure policy

7.1 Hard caps
• cas.max_blob MUST be <= WS gateway max_frame_bytes
• cas.max_provide_entries default: 64
• cas.max_want_hashes default: 65,536 (same as earlier CAS wire defaults)

7.2 Rate limiting (recommended)

Hub maintains:
• per-session “bytes in flight”
• per-session “blobs in flight”
• per-session “WANT rate” (hashes/sec)

Exceed → E_CAS_RATE_LIMIT or disconnect.

⸻

1. Versioning strategy (no footguns)

You must version both the session envelope and the CAS payloads.

8.1 Session envelope versioning
• JS-ABI VERSION stays as-is unless you change framing.
• OpEnvelope is CBOR; op names are stable.

8.2 CAS wire versioning

CAS bytes carry:
• magic ("WANT", "PROV", "CFRM", "CFRP")
• u16 version

So you can rev CAS independently from session wire.

8.3 Compatibility guarantee

A hub that advertises cas:ref-first:v1 MUST accept v1 CAS messages and reject non-v1 deterministically.

⸻

1. Tests (must add)

You already have integration test scaffolding in the TTD plan (handshake/subscription flow, etc.). ￼ Add these:

9.1 Session-level roundtrip
• encode Message::CasWant → decode back → bytes identical

9.2 Hub dedupe correctness
• two clients WANT same hash:
• only one upstream fetch
• both get satisfied

9.3 Provide verification
• PROV with wrong bytes:
• hub rejects entry
• does not store
• replies E_CAS_BAD_WIRE or E_CAS_INTERNAL (pick one; I recommend E_CAS_BAD_WIRE)

9.4 Limits enforced
• PROV blob too large → E_CAS_PAYLOAD_TOO_LARGE
• too many entries → E_CAS_PAYLOAD_TOO_LARGE
• WANT too many hashes → E_CAS_PAYLOAD_TOO_LARGE

9.5 CFRP equivalence
• CasFramePlus behaves identically to receiving:
• CasFrame
• then CasProvide

⸻

1. Why CAS ops are session-wire ops (and not EINT)

Because you already architected the session wire as a “transport/control plane” and the intent wire as a strict domain protocol. ￼ CAS is transport substrate — it belongs with session, not with intent.

⸻

TL;DR implementation checklist 1. Add Message::Cas\* variants + op_name mappings. 2. Add decode/encode branches for op strings. 3. Add CasBytesPayload { bytes } payload type. 4. Hub:
• parse CAS bytes
• verify hashes on provide
• blob cache + dedupe wants
• enforce limits + rate limits 5. Handshake capabilities:
• cas:ref-first:v1
• advertise limits in session_meta
