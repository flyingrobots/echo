<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ABI Golden Vectors (v1)

Status: Partial Rust-side vector set

These vectors document the **Canonical CBOR subset** used by the WASM ABI.
Rust-side encoder/decoder checks live in
`crates/echo-wasm-abi/tests/canonical_vectors.rs`; the canonical JS mapping is
specified in [`../js-cbor-mapping.md`](../js-cbor-mapping.md). Host-side
conformance vectors still need to be expanded before this can be treated as a
complete cross-language ABI lock.

## 1. Scalars

| Value   | Hex Encoding | Description      |
| :------ | :----------- | :--------------- |
| `null`  | `f6`         | CBOR Null        |
| `true`  | `f5`         | CBOR True        |
| `false` | `f4`         | CBOR False       |
| `0`     | `00`         | Smallest Int     |
| `-1`    | `20`         | Smallest Neg Int |
| `23`    | `17`         | Boundary Int     |
| `24`    | `18 18`      | 1-byte Int       |
| `255`   | `18 ff`      | 1-byte Int Max   |
| `256`   | `19 01 00`   | 2-byte Int       |

## 2. Strings (UTF-8)

| Value    | Hex Encoding     | Description   |
| :------- | :--------------- | :------------ |
| `""`     | `60`             | Empty string  |
| `"a"`    | `61 61`          | 1-char string |
| `"echo"` | `64 65 63 68 6f` | 4-char string |

## 3. Maps (Sorted Keys)

Maps MUST be sorted by the bytewise representation of their encoded keys.

### Example: `{ "b": 1, "a": 2 }`

- Key `"a"` encodes to `61 61`
- Key `"b"` encodes to `61 62`
- Correct Order: `"a"`, then `"b"`
- **Hex**: `a2 61 61 02 61 62 01`

### Test-backed vector: `{ "a": 1, "b": true }`

This vector is enforced by `golden_sample_map` in
`crates/echo-wasm-abi/tests/canonical_vectors.rs`.

- Key `"a"` encodes to `61 61`
- Key `"b"` encodes to `61 62`
- **Hex**: `a2 61 61 01 61 62 f5`

## 4. Nested Structures

### AppState Sample

```json
{
    "theme": "DARK",
    "navOpen": true,
    "routePath": "/"
}
```

**Canonical Sort Order** (bytewise on encoded keys):

1. `"theme"` (`65 74 68 65 6d 65`)
2. `"navOpen"` (`67 6e 61 76 4f 70 65 6e`)
3. `"routePath"` (`69 72 6f 75 74 65 50 61 74 68`)

**Hex Encoding**:
`a3` (map of 3)
`65 74 68 65 6d 65` ("theme") `64 44 41 52 4b` ("DARK")
`67 6e 61 76 4f 70 65 6e` ("navOpen") `f5` (true)
`69 72 6f 75 74 65 50 61 74 68` ("routePath") `61 2f` ("/")

**Full**: `a3 65 74 68 65 6d 65 64 44 41 52 4b 67 6e 61 76 4f 70 65 6e f5 69 72 6f 75 74 65 50 61 74 68 61 2f`
