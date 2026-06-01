<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Dynamic Footprint Binding Runtime

- Lane: `up-next`
- Legend: `KERNEL`
- Rank: `1`

## Why now

Echo now has the first compile-checked proof that a Wesley-generated bounded
rewrite interface can prevent undeclared capability access.

That proof still assumes a flat footprint. Real hot-graph rewrites need
dynamic binding:

- direct slot binding from args
- relation-based slot binding
- closure derivation over runtime graph truth

Without an explicit runtime model for those bindings, the stack risks either
freezing at toy footprints or reopening ad hoc traversal in handwritten Rust.

## Hill

Echo defines the runtime binding model for structured footprints so that:

- Wesley owns static slot/closure grammar
- Echo owns concrete binding and closure resolution
- implementations still cannot escape the declared capability surface

## Done looks like

- one Echo design note states the static-schema / dynamic-binding split
- one runtime-facing backlog item names the binding steps:
    - bind direct slots
    - bind relation-derived slots
    - resolve declared closures
    - enforce cardinality/basis validity
- one motivating rewrite shape, such as `ReplaceRangeAsTick`, is described in
  those terms
- the next runtime proof slice is obvious: bind one structured rewrite without
  reopening arbitrary traversal

## Repo Evidence

- `docs/invariants/DECLARATIVE-RULE-AUTHORSHIP.md`
- `docs/design/0012-dynamic-footprint-binding-runtime.md`
- `docs/method/backlog/up-next/PLATFORM_footprint-honesty-rewrite-proof-slice.md`
- `crates/echo-wesley-gen/tests/rewrite_api_contract.rs`
