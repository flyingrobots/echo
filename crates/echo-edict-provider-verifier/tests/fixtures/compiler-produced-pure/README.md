<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Compiler-produced pure relation regression

The package hex is copied byte-for-byte from Echo commit
`8c725d699241a7e3adee482029031ff6bade25fa`, fixture
`crates/warp-core/tests/fixtures/edict-pure-jedit/executable-operation-package.cbor.hex`.
It was produced by the public Edict compiler at
`3f81f759e921a69b04fe8cf8e62f8f3dc7b7e` and Echo provider
`49e9efb68001dfd78563d18bac9359a87671e431`.
The retained source, lawpack, adapter, and configuration are copied from Jim
`a894c7c4c6d150c0fb210d2e0ca4c27bf518b4c7`, under `edict/replace-range/`.
Package raw SHA-256: `1860ab8a6cb9a4bfd63a1db99d1c2bc00aa53dae52a9d1998295654379a95d85`.

The positive control requires the native lowerer to reproduce the exact original
package and the separate verifier to accept it. The adversarial case changes a
U32 literal in Target IR while retaining the original Core, then rebinds the
package normally. A semantic verifier must refuse that inconsistent relation.
This is a compiler-artifact mutation test, not an authored source change or
proof of an admitted Jim edit. Application vocabulary appears only in fixtures.
