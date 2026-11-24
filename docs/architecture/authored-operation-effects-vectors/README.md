<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Bridge Action byte vectors

These vectors give exact expected bytes or digests for the identities,
digests and retained encodings that phase 2 of
[Authored Operation Effects](../authored-operation-effects.md#phase-2-acceptance-plan)
writes. They pin layouts the owner has not yet approved. If a layout in the
document changes, regenerate the vectors.

- `generate_bridge_vectors.py` builds every value from the layouts written in
  the document. It uses only the Python standard library, lives outside
  `warp-core` and shares no code with it.
- `blake3_reference.py` is a BLAKE3 implementation written from the BLAKE3
  specification. The generator checks it against the official BLAKE3 test
  vectors (lengths 0, 1024, 1025 and 2048) and emits nothing if a check fails.
- `bridge-vectors.json` is the generator's output. Each vector records its
  inputs and the source of its expected value.

Regenerate and check that nothing changed:

```sh
python3 -I docs/architecture/authored-operation-effects-vectors/generate_bridge_vectors.py \
  | cmp - docs/architecture/authored-operation-effects-vectors/bridge-vectors.json
```

## Coverage

The vectors cover these new encodings:

- the synthetic receipt scope and its `scope_hash`;
- the node, atom, portal and edge value digests;
- `SlotRow`, `ReadRow`, `EdgePairRow` and `EffectRow`;
- the read-set digest, including the empty set, and the target-set and
  write-set digests;
- the `EBPRP001` proposal image and the proposal identity;
- `authored_id` and `result_identity`;
- `OutcomeRow`, and a `BridgeEntryRow` for `NotBridge` and for failure-matrix
  rows D, G, K, L, N and O;
- the 152-byte decision coordinate, and `EOACT004` in its committed,
  obstructed and conflict variants;
- the `EPRV0004` record, the receipt digest version 5 preimage and digest;
- the `:v2` rewrites and composition digests, both for a mixed Tick and for a
  bridge Tick in which only the fixed member applied.

They also cover these existing recipes:

- the `:v1` rewrites and composition digests of a fixed-only Tick;
- receipt digest version 4 over the same entries.

Both are re-implemented here from the recipes in `warp-core` at `53c78e8a`,
so the generator can show that a fixed-only Tick keeps its bytes.

Some encodings are out of scope:

- Scoped-validity rows (`DependencyRow`, and the `origin_basis`,
  `dependency_set`, `compared_values` and `changed_dependency` fields) are
  zero in this profile. Phase 2 does not write them.
- Content-address ids, package, program and profile identities, and the
  application-basis value identity already exist and keep their own vectors
  (for example `crates/warp-core/tests/fixtures/content-address-v1/`).
- The `EPRV0003` payload inside `EPRV0004` is an opaque input that phase 2
  copies byte for byte.

## Status

The planned warp-core test `bridge_encodings_match_independent_vectors` will
compare the production encoders against this file. That test does not exist
yet. No production encoder has been checked against these vectors.
