<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Source-module effect compiler request

These are the unmodified canonical artifacts of the lowering request that
Amblicit `738d3da7` sent to Echo's lowerer when it built the application in
[`crates/warp-core/tests/fixtures/edict-effect-source-module/source`](../../../../warp-core/tests/fixtures/edict-effect-source-module/source)
(flyingrobots/echo#1009), repinned to the `echo.dpo@2` profile `7cadb97e`
after echo#1014. The entry module calls `leaf.same(input.v)` after an
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
| `01-lawpack-adapter.hex` | 1160 | `b2056f9bf22d649600ad187a262346d8e6bd93099dfeed29b098b15759f3c8c8` |
| `02-lawpack-exports.hex` | 1890 | `0c66914501400f138eb2746df7da17c9359d366dc20b73172cea8ff71e172928` |
| `03-lawpack.hex` | 817 | `78c7c06c4e97f19411a1a1e5a3b17d7554a9f47438e7fcd50c7d65e0b748b85e` |
| `04-source.hex` | 932 | `a0dcccfa2693dcdd39207481000eb633d042a9b9b9978e95f1efb95d545ab8ff` |
| `05-target-configuration.hex` | 6589 | `5b10d3b1e4d0f167f049b8ac597e4780ca3eee82d3d13a23496c2c02cdcbe97e` |
| `06-target-ir.hex` | 1750 | `ba970a098e97d8aec73f7742f29be5163018b19dcf499b347f8a8f93a035f84a` |
| `07-result-projection.hex` | 256 | `fa698723ac9408f98ee29430a06320e7d2a9c180c5037f3ee2304dc66990010a` |
| `core.hex` | 2256 | `12aeeb467857d5e1cbfd164b0f03a7b1022e359be60f710ff07ffdeb96349d26` |
| `profile.hex` | 1787 | `8041a58003593571e546302bb1736b24c0ef8257b5fe3651023d618fe666de4d` |
