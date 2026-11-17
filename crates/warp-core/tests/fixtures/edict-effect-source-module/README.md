<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Source-module effect fixture

This application calls a helper from a digest-pinned source module in an
`echo.dpo@2` effect program (flyingrobots/echo#1009). `source/Program.edict`
runs one `ensureNode` effect, then `leaf.same(input.v)`. `same` is
`fn same(v: U64) -> U64 { return v; }` in `source/modules/Leaf.edict`
(`jedit.probe.leaf@1`), which the entry module pins with
`use module jedit.probe.leaf@1 digest "sha256:83ed90b9…" as leaf;`.

Amblicit links `same` into the one Core module under its linked name, so the
three judges check it as they check an entry helper. The lowering artifacts
retain the `edict.source-closure/v1` envelope of both modules, and each judge
requires the entry module's `use module` pin to name the retained leaf by its
exact `edict.source/v1` identity.

`source-module-executable-operation-package.hex` is the lowerer's package for
the compiler request retained in
[`crates/echo-edict-provider-lowerer/tests/fixtures/source-module-effect`](../../../../echo-edict-provider-lowerer/tests/fixtures/source-module-effect).
The shared provider corpus retains these bytes unchanged as
`source-module-control`, and derives its two forged closures from them
(`tests/edict_effect_package_tests/source_modules.rs`). Run with `v = 7`, the
program returns `{ b: 7 }`.
