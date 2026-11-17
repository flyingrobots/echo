<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Source-module effect compiler request

These are the unmodified canonical artifacts of the lowering request that
Amblicit `738d3da7` sent to Echo's lowerer when it built the application in
[`crates/warp-core/tests/fixtures/edict-effect-source-module/source`](../../../../warp-core/tests/fixtures/edict-effect-source-module/source)
(flyingrobots/echo#1009). The entry module calls `leaf.same(input.v)` after an
`ensureNode` effect; `same` lives in the source module `jedit.probe.leaf@1`,
which the entry module pins by digest. The source input is therefore an
`edict.source-closure/v1` envelope, and Core carries the helper under its
linked name.

For capture only, the compiler was instrumented immediately before it invoked
the lowerer component: it wrote `request.core`, `request.target_profile` and
every semantic input's artifact bytes to a file. This adds observation to the
compiler and changes no compilation or validation decision. The hex files
retain those exact bytes. `tests/source_module_effect.rs` rebuilds the role and
coordinate envelopes and recomputes each artifact's canonical identity.

At Echo `93684efd` the lowerer refused this request with
`UnsupportedSemantics` ("the compiled effect program is outside the checked
profile"). The same program with `same` defined in the entry module was
accepted.

| Artifact | Bytes | Raw SHA-256 |
| --- | ---: | --- |
| `01-lawpack-adapter.hex` | 1160 | `64e5a72d23a04f46cd4cf379dbc5c9ea625e8c1a54cb1d7c004fd5533ba198aa` |
| `02-lawpack-exports.hex` | 1890 | `0c66914501400f138eb2746df7da17c9359d366dc20b73172cea8ff71e172928` |
| `03-lawpack.hex` | 817 | `4336ecb5be26afeafea90c19be9c3c1a31534cf612d50b6dc04a2e626172865b` |
| `04-source.hex` | 932 | `c9f39ea1384083eefde5c29ea84afba9352f2ab106c0bfc9ba1cb6c6ddad42e5` |
| `05-target-configuration.hex` | 6222 | `eb3e4cf0de7f537aa0e45e43e1af75ad17dcfc466499e1d3ca4063669ad2e690` |
| `06-target-ir.hex` | 1750 | `c302d6cd0e372b26d570198260b0e0c269368384fcf8d5a6656e906eea9f5131` |
| `07-result-projection.hex` | 256 | `fa698723ac9408f98ee29430a06320e7d2a9c180c5037f3ee2304dc66990010a` |
| `core.hex` | 2256 | `c8db543b3569693469f75854917a4ba170f41fc94756c4a2ecdd18528f5986e7` |
| `profile.hex` | 1787 | `b40970846285d4a94fde86e6dc600deaccbc1cb2dc9aa3ec6437eebc8483d5a4` |
