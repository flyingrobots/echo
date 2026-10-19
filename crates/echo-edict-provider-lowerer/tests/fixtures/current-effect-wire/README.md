<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Current effect wire controls

These are synthetic serialization inputs, not public compiler outputs. They
contain a zero-effect integer-result Core/ordered-IR control, one declared
obstruction, graph/footprint metadata, explicit limits and a source byte string
that says `synthetic serialization control`.

They were materialized from the runtime-owner fixture helpers in
`crates/warp-core/tests/common/edict_effect_publication.rs` at
`41037d48b66fea9e88af1de96a502d2b1654b77c`, with the explicit effect
configuration and obstruction added by the lowerer serialization witness.
Each `.hex` file contains the canonical bytes of one whole input artifact.
Copies live in this crate so its source archive does not depend on another
crate's test files.

The witness checks deterministic current-format serialization and the current
runtime decoder's exact round trip. It does not establish compiler correctness,
policy or profile authority, independent verifier acceptance, installed Action
execution, settlement or recovery. The public lowerer does not yet call this
serializer.
