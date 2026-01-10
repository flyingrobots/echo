<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# echo-wasm-abi

Shared WASM-friendly DTOs for Echo/JITOS living specs. Mirrors the minimal graph + rewrite shapes used in Spec-000 and future interactive specs.

## Types

- `Node`, `Edge`, `WarpGraph`
- `Value` (Str/Num/Bool/Null)
- `Rewrite` with `SemanticOp` (AddNode/Set/DeleteNode/Connect/Disconnect)
- Deterministic CBOR helpers: `encode_cbor` / `decode_cbor` (canonical subset, no tags/indefinite)

## Usage

Add as a dependency and reuse the DTOs in WASM bindings and UI code to keep the schema consistent across kernel and specs.

### Canonical encoding
- `encode_cbor` / `decode_cbor` use the same canonical CBOR rules as `echo-session-proto` (definite lengths, sorted map keys, shortest ints/floats, no tags).
- Integers are limited to i64/u64 (CBOR major 0/1); float widths are minimized to round-trip.
- Host code should call into Rust/WASM helpers rather than hand-encoding bytes to avoid non-canonical payloads.
