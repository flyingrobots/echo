<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0017: Universal Little-Endian Codec

- **Status:** Accepted
- **Date:** 2026-07-13

## Context

Echo crosses Rust, WASM, and JavaScript boundaries. Platform-native layout,
implicit widths, and host-specific float conversion would make identity and
replay depend on the implementation language.

## Decision

Fixed-width binary fields use canonical little-endian encoding. Variable data
is length-delimited, versions and discriminants are explicit, and decoders
reject trailing, truncated, over-budget, unknown-version, and invalid-enum
input. Cross-language golden vectors are part of the contract.

Canonical CBOR surfaces retain their own deterministic subset rules; this ADR
does not replace the CBOR specification.

## Consequences

- No native struct layout or platform endianness crosses a public boundary.
- Encoders and decoders must agree byte-for-byte across supported languages.
- Float-to-fixed conversion policy requires explicit golden vectors rather than
  host-language casts.

## Evidence Anchors

- `docs/spec/SPEC-0009-wasm-abi.md`
- `docs/spec/js-cbor-mapping.md`
- `docs/spec/abi-golden-vectors.md`
- `crates/echo-wasm-abi/src/codec.rs`
