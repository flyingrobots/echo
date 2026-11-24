<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Public compiler graph-read witnesses

`source/` holds authored Amblicit input only: a lawpack exporting two read
effects bound to `echo.dpo@1.outgoing-edge-read` and `echo.dpo@1.closure-read`,
and one program for each. The coordinate `consumer.graph` is neutral; Echo
interprets none of it. The closure program's `select` helper is the guided
selector. It follows two offsets down a tree whose branch atoms carry their
split, so one read covers both boundary paths of an edit.

The hexadecimal files are unchanged canonical CBOR written by the public
compiler, Amblicit `66bf6bac369f67e32ddaee099c3198bd30f885a1` (CI now reproduces them
byte for byte with `0034605467fcace397e7e30e4219a74aca57f183`), through the
checked provider package in `schemas/edict-provider/package/v1`. Nothing here
was decoded and re-encoded, and no Core or Target IR was authored by hand. The
authored programs carry the digest of the lawpack that the same compiler built
from `source/edict.lawpack.json`.

`scripts/verify-edict-provider-host-v1.sh` rebuilds these bytes in CI with
`xtask --example graph_read_publication --check`. It builds the lawpack,
compiles each program twice, and requires the output to equal these files and
the predecessor provider package to refuse the closure program.

| Program | Package domain-framed digest (`echo.operation-package/v1`) |
| --- | --- |
| Edge | `1f2ef3ce45822995465e9250f8a98083e23dbf2c1cd682f5372826099580b926` |
| Closure | `401680675b8364415b2d6e0768ba386ccc7085ad40b3c9e0021ed87bb1b90e2c` |

The runtime tests in `crates/warp-core/tests/edict_node_read/public_graph_reads.rs`
fix these pins independently of the bytes and evaluate the packages over real
frontier storage. The provider tests in
`crates/echo-edict-provider-verifier/tests/bounded_read/graph_reads.rs` rebuild
each lowering request from the package's embedded artifacts and require the
native lowerer and verifier to reproduce these bytes.

These packages are private bounded-read evaluation witnesses. They do not
establish installation, a causal Tick, WAL retention or a receipt.
