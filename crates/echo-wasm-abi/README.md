<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# echo-wasm-abi

Shared WASM-friendly DTOs for Echo/JITOS living specs. Mirrors the minimal graph + rewrite shapes used in Spec-000 and future interactive specs.

## Types

- `Node`, `Edge`, `Rmg`
- `Value` (Str/Num/Bool/Null)
- `Rewrite` with `SemanticOp` (AddNode/Set/DeleteNode/Connect/Disconnect)

## Usage

Add as a dependency and reuse the DTOs in WASM bindings and UI code to keep the schema consistent across kernel and specs.
