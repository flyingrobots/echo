<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Frozen ContentAddressV1 vectors

These six files contain the independently derived identities checked by
[`content_address_v1_tests.rs`](../../content_address_v1_tests.rs). Each file is
one lowercase, 32-byte hexadecimal digest followed by a newline. They preserve
the values previously embedded in that test; moving them does not change the
identity law or establish public compiler/provider/runtime support.

The test-side [preimage builder](../../common/content_address_preimage.rs)
assembles each domain and field independently of the production identity
module. The conformance test compares that preimage's BLAKE3 digest with both
the frozen value and the production result. Consumers must compare their
result with these files, rather than regenerate their expected values from
the implementation under test.

All identifiers below are 32 repetitions of the named byte. The atom is the
three raw bytes `6e6577` (`new`), not a CBOR string. Warp is `03`, node type is
`07`, atom type is `02`, edge type A is `06`, edge type B is `08`, low target
is `04`, and high target is `05`.

| File | Identity inputs beyond the common values |
| --- | --- |
| `attachment-new.hex` | Atom type and atom bytes |
| `node-zero-edges.hex` | No outgoing edges |
| `node-one-edge.hex` | One outgoing edge: A to high |
| `node-two-edges.hex` | Input order B to low, then A to high; canonical order A to high, then B to low |
| `content-edge.hex` | Source low, edge type A, target high |
| `slot-edge.hex` | Source low, edge type A; no target input |

The canonical [identity law](../../../../../docs/architecture/authored-operation-effects.md#identity-laws)
owns domain separators, framing, sorting, and duplicate-edge refusal. These
vectors cover successful identities; the companion tests also check input
mutations, duplicate refusal, order invariance, and diagnostic disclosure.
