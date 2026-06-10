<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Continuum Proof Family Runtime Cutover

- Lane: `up-next`
- Legend: `PLATFORM`
- Rank: `1`

## Why now

Echo now publishes real neighborhood and settlement proof surfaces, but they
are still handwritten Echo-side DTOs:

- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-core/src/neighborhood.rs`
- `crates/warp-core/src/settlement.rs`
- `crates/warp-wasm/src/warp_kernel.rs`

That is useful as a proof runway, but it is not the assignment. The assignment
is:

1. Continuum declares the shared family
2. Wesley compiles it
3. Echo runs against the generated Rust side
4. `warp-ttd` consumes the generated TypeScript side

## Hill

Replace the proof-slice handwritten Echo boundary with Wesley-generated Rust
artifacts and keep the runtime semantics the same.

For the first cut, the proof slice should cover:

- neighborhood core publication
- reintegration / settlement publication
- one rewrite op with declared footprint
- one valid implementation
- one invalid compile-fail implementation
- one generated artifact hash / footprint certificate that Echo can check at
  load time

## Done looks like

- Echo compiles one proof slice against Wesley-generated Rust artifacts
- handwritten proof-slice ABI/runtime DTOs are removed or proven isomorphic
  temporary shims
- one invalid rewrite that exceeds its declared footprint fails to compile
- the generated proof slice exposes a stable artifact hash or certificate hash
  that Echo can compare before trusting the optimized path
- runtime guards remain as second-line safety, not the only proof
- the browser/WASM host bridge is still able to publish the resulting proof
  family

## Repo Evidence

- `docs/design/0006-echo-continuum-alignment/design.md`
- `docs/design/0007-braid-geometry-and-neighborhood-publication/design.md`
- `docs/design/0008-strand-settlement/design.md`
- `crates/echo-wasm-abi/src/kernel_port.rs`
- `crates/warp-core/src/neighborhood.rs`
- `crates/warp-core/src/settlement.rs`
