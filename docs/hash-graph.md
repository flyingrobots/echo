# Hash Graph Overview

Echo uses content-addressed hashing to provide provenance and deterministic replay. This document maps how hashes relate across subsystems.

---

## Root Manifest
- `manifestHash = BLAKE3(sorted(nodeHashes || snapshotHashes || diffHashes || payloadHashes))`
- Records top-level references for branch nodes, snapshots, diffs, payloads.

## Config Hash
- `configHash = BLAKE3(canonical(config.json))`
- Stored in block manifest and determinism logs.
- Replay verifies configHash before executing diffs.

## Plugin Manifest Hash
- Each plugin manifest hashed; combined `pluginsManifestHash = BLAKE3(sorted(manifestHashes))`.
- Stored in manifest along with plugin registry version.

## Schema Ledger Hash
- `schemaLedgerHash` ties component layouts to snapshots.

## Diff & Snapshot Hash
- Diffs and snapshots hashed via serialization protocol (see spec-serialization-protocol.md).

## Event Envelope Hash
- `envelopeHash = BLAKE3(canonical event bytes)` used for dedup, signatures, and causality.

## Composition
```
manifestHash
├─ configHash
├─ pluginsManifestHash
├─ schemaLedgerHash
├─ snapshotHash
│   └─ chunkRefHashes
├─ diffHash
│   └─ chunkDiff payload hashes
└─ eventEnvelopeHashes (if persisted)
```

These hashes ensure each phase of the simulation can be verified independently and recombined deterministically.
