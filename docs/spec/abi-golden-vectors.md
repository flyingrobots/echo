<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ABI Golden Vectors

_Define the canonical byte examples that keep host encoders and runtime decoders aligned._

Legend: PLATFORM

Status: Partial Rust-side vector set

Depends on:

- [SPEC-0009 - WASM ABI Contract](SPEC-0009-wasm-abi.md)
- [JS to Canonical CBOR Mapping](js-cbor-mapping.md)

## Why this packet exists

The ABI is a byte contract. Golden vectors are the shared evidence that independent encoders emit the same bytes for the same logical value.

## Human users / jobs / hills

Human users need ABI failures to be diagnosable.

The hill: when a host adapter changes, a reviewer can compare expected hex with actual hex and identify whether the drift is key ordering, integer width, definite length, or schema shape.

## Agent users / jobs / hills

Agent users need compact fixtures for conformance checks.

The hill: an agent can generate a payload, compare it against these vectors, and decide whether it is safe to cross the WASM boundary.

## Decision 1: Golden vectors are evidence, not a second spec

The mapping rules live in [JS to Canonical CBOR Mapping](js-cbor-mapping.md). This packet gives executable examples aligned with `crates/echo-wasm-abi/tests/canonical_vectors.rs`.

## Decision 2: Scalar vectors cover shortest-width encoding

| Value   | Hex        | Meaning                     |
| ------- | ---------- | --------------------------- |
| `null`  | `f6`       | CBOR null                   |
| `true`  | `f5`       | CBOR true                   |
| `false` | `f4`       | CBOR false                  |
| `0`     | `00`       | smallest unsigned integer   |
| `-1`    | `20`       | smallest negative integer   |
| `23`    | `17`       | one-byte major-type payload |
| `24`    | `18 18`    | uint8 boundary              |
| `255`   | `18 ff`    | uint8 max                   |
| `256`   | `19 01 00` | uint16 boundary             |

## Decision 3: Map vectors cover encoded-key sorting

Maps sort by the encoded CBOR key bytes. For `{ "b": 1, "a": 2 }`, canonical hex is `a2 61 61 02 61 62 01`. For `{ "a": 1, "b": true }`, canonical hex is `a2 61 61 01 61 62 f5`.

## Decision 4: Nested vectors must make order visible

For `{ "theme": "DARK", "navOpen": true, "routePath": "/" }`, canonical key order is `theme`, `navOpen`, `routePath` by encoded key bytes.

Canonical hex: `a3 65 74 68 65 6d 65 64 44 41 52 4b 67 6e 61 76 4f 70 65 6e f5 69 72 6f 75 74 65 50 61 74 68 61 2f`.
