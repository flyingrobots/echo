<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# echo-wesley-gen is a 2000-line monolith with mixed concerns

Status: bad code.

## Where

`crates/echo-wesley-gen/src/main.rs` — single 2000+ line file.

## Smell

One file emits:

- GraphQL → Rust type definitions (struct/enum)
- LE binary `Encode`/`Decode` impls
- Op id constants + `OP_*_ARGS` tables
- Footprint certificates + observer plan identities
- `RegistryProvider` implementation + static `REGISTRY`
- `__echo_wesley_generated` helper module with `Vars` structs
- EINT intent packers (`pack_*_intent`, `pack_*_intent_raw_vars`)
- Optic dispatch/observe request builders
- Contract-host helpers (`*_contract_rule`, `*_contract_vars`,
  `*_query_observer`) gated by `--contract-host`

Every consumer pays the cost of every concern. Tests are slow (full
cargo run + smoke crate compile per case). Adding a new emit feature
means threading a new flag through eight code paths.

## Why it matters

This file is also where the cross-language `stable_op_id` algorithm
lived until 2026-05-28. The fact that algorithm-as-source-of-truth
got buried in a code-emitter monolith is precisely the kind of smell
that delayed Wesley adoption.

## Suggested split

- `echo-wesley-gen-types` — struct/enum + Encode/Decode emit
- `echo-wesley-gen-ops` — op_id table + arg descriptors + RegistryProvider
- `echo-wesley-gen-intent` — EINT packers + optic helpers
- `echo-wesley-gen-contract-host` — gated host-side helpers
- A thin top-level binary that composes them

Each compose-able piece can be tested in isolation. Algorithms shared
across targets (e.g., `stable_op_id`) migrate to `wesley-core` rather
than a vendored copy.

## Surface when

Touching anything in this file. The next agent who has to add an emit
target will feel the pain; that's a good moment to extract.
