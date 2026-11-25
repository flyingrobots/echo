<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Source-module effect compiler request

These are the unmodified canonical artifacts of the lowering request that
Amblicit `738d3da7` sent to Echo's lowerer when it built the application in
[`crates/warp-core/tests/fixtures/edict-effect-source-module/source`](../../../../warp-core/tests/fixtures/edict-effect-source-module/source)
(flyingrobots/echo#1009), repinned to the `echo.dpo@2` profile `7cadb97e`
after echo#1014, and recaptured for the `/v3` profile `4e60b129` (echo#1016):
`05-target-configuration.hex` is the configuration Amblicit `aa8c7cbf`
retained for that witness, and every other changed file is the matching
artifact carried in the package that compiler wrote for it (the bundle's
Core, Target IR and exports, and its lowering artifacts), which equal the
request bytes the lowerer receives; each of the seven was checked to equal
the earlier capture when taken from the earlier package. The entry module calls `leaf.same(input.v)` after an
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
| `01-lawpack-adapter.hex` | 1160 | `55b1fb6f6aae278a26af61162904b3f3c5c36e6933842b2c1a4148206e1e5887` |
| `02-lawpack-exports.hex` | 1890 | `0c66914501400f138eb2746df7da17c9359d366dc20b73172cea8ff71e172928` |
| `03-lawpack.hex` | 817 | `2d530d5dbed5d83d6a5b77c95cae35fc5eef96a3320b0c0a6a7b089000ffa46d` |
| `04-source.hex` | 932 | `d332ee9782146c3711a50bc2f0343d416e780bc7f4b8a61eecea2f92fae3a68c` |
| `05-target-configuration.hex` | 10954 | `d378846bd21f8ce518936f2f6723ebbc36593741013b6eeafac4cf1b18a5ac17` |
| `06-target-ir.hex` | 1750 | `3b8017471ddb561ee94cbdd1484295fdb1e5a3335bf90d07c2cfe88677339468` |
| `07-result-projection.hex` | 256 | `fa698723ac9408f98ee29430a06320e7d2a9c180c5037f3ee2304dc66990010a` |
| `core.hex` | 2256 | `c451acb41f874d37a471b5f8e3fa63d7daa9e869106c353dc4b40a7fe3df788e` |
| `profile.hex` | 1787 | `c0955185f214da50a071cb387bbfeb128202ae92b1e4d973d88fa7dc5323113a` |
