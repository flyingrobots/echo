<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

> **Milestone:** [Deep Storage](README.md) | **Priority:** P2

# Wire Protocol

WANT/PROVIDE/FRAME message types for network-level blob exchange between peers.

---

## T-5-3-1: Message type definitions and binary encoding

**User Story:** As a developer, I want a compact binary wire format for blob exchange so that peers can request and transfer blobs efficiently.

**Requirements:**

- R1: Define message types: `Want { hash: BlobHash }`, `Provide { hash: BlobHash, size: u64 }`, `Frame { hash: BlobHash, offset: u64, data: Vec<u8> }`, `Have { hash: BlobHash }`, `Error { code: u16, message: String }`.
- R2: Binary encoding: 1-byte message tag + length-prefixed fields. All integers little-endian.
- R3: Implement `encode(&self) -> Vec<u8>` and `decode(bytes: &[u8]) -> Result<Message, WireError>` for each type.
- R4: Maximum message size: 1MB (enforced in `decode`).

**Acceptance Criteria:**

- [ ] AC1: Round-trip: `decode(encode(msg)) == msg` for all message types.
- [ ] AC2: `decode` of truncated bytes returns `WireError::Incomplete`.
- [ ] AC3: `decode` of a message exceeding 1MB returns `WireError::TooLarge`.
- [ ] AC4: Binary encoding is stable: golden vectors checked in for each message type.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Message type definitions, encode/decode, golden vectors, error types.
**Out of Scope:** Transport layer (TCP/WebSocket). Connection management. Authentication.

**Test Plan:**

- **Goldens:** Hex-encoded golden vectors for each message type (Want, Provide, Frame, Have, Error).
- **Failures:** Unknown message tag. Truncated at every byte position. Oversized message. Invalid UTF-8 in Error message.
- **Edges:** Frame with 0-byte data. Want for zero-hash. Provide with size=0.
- **Fuzz/Stress:** Proptest: encode random messages, decode, assert equality. Decode 100,000 random byte slices (must not panic).

**Blocked By:** none
**Blocking:** T-5-3-2

**Est. Hours:** 5h
**Expected Complexity:** ~300 LoC

---

## T-5-3-2: Request/response protocol and backpressure

**User Story:** As a developer, I want a protocol state machine that handles blob exchange with flow control so that network transfers do not overwhelm memory.

**Requirements:**

- R1: `BlobExchange` struct manages the protocol state: pending wants, in-flight provides, received frames.
- R2: Outbound: `request(hash)` enqueues a `Want`. When a `Provide` arrives, send `Want` again to confirm (pull model).
- R3: Inbound: on receiving `Want`, check local store and send `Provide` (or `Error::NotFound`). On confirmed `Want`, stream `Frame` messages.
- R4: Backpressure: limit in-flight concurrent wants to a configurable `max_concurrent` (default 16).
- R5: Frame reassembly: accumulate `Frame` chunks for a hash until `offset + data.len() == size`, then call `put_verified`.

**Acceptance Criteria:**

- [ ] AC1: Full blob transfer: peer A requests hash H from peer B; B streams frames; A reassembles and stores.
- [ ] AC2: In-flight limit: requesting 32 blobs with `max_concurrent=16` results in 16 active wants, 16 queued.
- [ ] AC3: Hash mismatch on reassembled blob (corrupted frame) surfaces as `CasError::HashMismatch`.
- [ ] AC4: Timeout: want with no response after configurable duration triggers retry or error.

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Protocol state machine, backpressure, frame reassembly, `put_verified` integration.
**Out of Scope:** Transport binding (TCP/WS). Discovery/peer management. Encryption.

**Test Plan:**

- **Goldens:** N/A (stateful protocol -- tested behaviorally).
- **Failures:** Provide for a hash we never wanted (ignored). Frame for unknown hash (ignored). Frame with offset beyond declared size.
- **Edges:** 1-byte blob (single frame). Blob exactly at 1MB boundary. Duplicate Want for same hash.
- **Fuzz/Stress:** Simulate 100 concurrent blob transfers with random frame ordering; all must complete successfully.

**Blocked By:** T-5-3-1
**Blocking:** none

**Est. Hours:** 6h
**Expected Complexity:** ~350 LoC
