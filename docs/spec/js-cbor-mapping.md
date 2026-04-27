<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# JS to Canonical CBOR Mapping

_Define how host-side JavaScript values become deterministic ABI bytes._

Legend: PLATFORM

Depends on:

- [SPEC-0009 - WASM ABI Contract](SPEC-0009-wasm-abi.md)
- [ABI Golden Vectors](abi-golden-vectors.md)

## Why this packet exists

At the ABI boundary, bytes are the identity surface. JavaScript values have ambient behaviors that are not deterministic enough for hashing, replay, or ledger stamping. This packet defines the allowed mapping into canonical CBOR.

## Human users / jobs / hills

Human users need host adapter bugs to fail before they enter the kernel.

The hill: malformed or non-canonical values are rejected at the boundary with a clear ABI error instead of being normalized differently by different hosts.

## Agent users / jobs / hills

Agent users need deterministic payload construction.

The hill: an agent can encode the same logical value twice and get byte-identical CBOR before submitting it as an intent or observation request.

## Decision 1: The supported scalar mapping is narrow

`null`, booleans, strings, safe integral numbers, and canonical floats are allowed. `undefined` and `BigInt` are rejected for the current ABI mapping.

## Decision 2: Bytes and arrays are definite-length only

`Uint8Array` maps to a definite-length byte string. JS arrays map to definite-length arrays with recursively encoded elements. Indefinite-length CBOR is never accepted or emitted.

## Decision 3: Objects map to sorted string-key maps

Object keys must be strings. Encoded map keys are sorted by their CBOR byte encoding, not insertion order. Duplicate keys, non-string keys, unknown fields, and missing required fields are schema errors.

## Decision 4: Prohibited CBOR features stay prohibited

The ABI mapping does not use tags, indefinite-length values, non-shortest integer encodings, or non-canonical float encodings.

## Decision 5: Canonical bytes are payload identity

The bytes produced by this mapping are the bytes used for hashing, ledger identity, EINT payloads, and observation requests.
