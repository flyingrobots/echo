<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# JS → Canonical CBOR Mapping Rules (ABI v1)

Status: **Frozen for website kernel P1**

These rules define how host-side JS/TS values must be mapped into the canonical CBOR
encoder before hitting the ledger. The same rules apply to wasm helpers
(`encode_command`, `encode_query_vars`) and to native tooling.

## Scalars
- `null` → CBOR null
- `boolean` → CBOR bool
- `string` → CBOR text (UTF-8)
- `number`
  - If integral and `abs(n) <= Number.MAX_SAFE_INTEGER` → CBOR integer (shortest width)
  - Else → CBOR float (smallest width that round-trips; NaN/±∞ allowed, canonicalized)
- **Ban** `undefined` (error)
- **Ban** `BigInt` for P1 (use string/bytes if needed)

## Bytes
- `Uint8Array` → CBOR byte string (definite length)

## Arrays
- JS array → CBOR array (definite length), elements encoded recursively.

## Objects (Maps)
- Keys **must** be strings; non-string keys are rejected.
- Encoded as CBOR map with keys sorted by their CBOR byte encoding (canonical).
- Duplicate keys are rejected.
- Unknown/extra fields should be rejected at schema validation (Zod/Rust).

## Prohibited CBOR features
- No tags.
- No indefinite-length strings, arrays, or maps.
- Shortest encodings required for ints/floats.

## Error surface (host-facing)
- INVALID_INPUT for: undefined, BigInt, non-string map keys, duplicate keys, unknown fields,
  missing required fields, non-canonical float/int widths, indefinite-length items.

## Canonical payload identity
- The exact CBOR bytes produced by these rules are the authoritative payload for hashing
  and ledger stamping. Re-encoding the same logical value must yield identical bytes.

## References
- `crates/echo-wasm-abi/src/canonical.rs` — canonical encoder/decoder and rejection tests.
- ADR-0013 / JS-ABI v1 framing (canonical CBOR payload inside packet header).
