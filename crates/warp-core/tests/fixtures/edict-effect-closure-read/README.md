<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Public compiler closure-read effect program under `echo.dpo@2`

`source/` holds authored Amblicit input only: a lawpack whose adapter accepts
the `echo.dpo@2` effect target profile and binds `fx.closure` to
`echo.dpo@2.closure-read`, with its seven failures mapped to authored
obstructions; and one program, `ClosureLink.edict`, that reads one guided
closure and returns it (echo#1016). Its `select` helper is the declared
selector (`graphReads.closureSelector`); `costly`, a costlier helper of the
same type, is never selected, so the copying schedule (which charges the
costliest helper) prices the read above the shared-read schedule (which
charges the declared selector). It follows slot 0 when the guide's
`followA` is set and slot 1 when `followB` is set, and uses no byte
operation, so every expression has a shared-read rule in Echo. The
coordinates under `consumer.closures` are neutral; Echo interprets none of
them.

The lawpack was derived from `edict-effect-graph-read/source` (the effect
envelope and Echo configuration) and `edict-graph-read/source` (the closure
types, effect and failures). Its configuration differs from the graph-read
effect's in these ways: `graphReads` allows 64 edge scans, 8 closure nodes and
depth 4 with selector `select`. The graph schema and footprint law admit atom
type `0x02…` on node type `0x6b…`, because a closure reads each node's atom.
The budgets and result bound are widened for the closure result.

The hexadecimal files are unchanged canonical CBOR written by the public
compiler, Amblicit `2b09e4ab1ecf0041b4ff251693dc295ac2b649c2`, through the
checked provider package in `schemas/edict-provider/package/v1`. Nothing here
was decoded and re-encoded, and no Core or Target IR was authored by hand.
`ClosureLink.edict` carries the digest of the lawpack the same compiler built
from `source/edict.lawpack.json`.

`scripts/verify-edict-provider-host-v1.sh` rebuilds these bytes in CI with
`xtask --example effect_step_publication --name closure --check`, which also
requires Echo's lowerer to refuse the same program under `echo.dpo@1` and
under a substituted `echo.dpo@2` profile.

Two tests use the package:

- `closure_read_witness_minimums_under_each_schedule` in
  `crates/echo-edict-provider-lowerer/tests/effect_program_judgment.rs` pins
  the lowerer's minimum budgets under the copying schedule and under
  `core.cost.shared-reads/v1`.
- `the_closure_witness_runs_within_the_lowerer_minimum` in
  `crates/warp-core/tests/edict_effect_package_tests/provider_corpus/closure_witness.rs`
  gives the package exactly that shared minimum. It reads an eight-node graph
  to depth four (the read's node and depth bounds) to the copying schedule's
  result and requires the runtime charge on both axes to be at or below the
  static bound. The compiled footprint law allows 16 reads and 8 range edges,
  too few for that read, so the test widens both to 64. The law's bytes are
  bound twice, inside the program and as the package's
  `footprint_contract_identity`; changing one without the other is what makes
  preparation fail with `InvalidPackage`, and the test repins both.

The package is a private evaluation witness. It does not establish
installation, a causal Tick, WAL retention or a receipt.
