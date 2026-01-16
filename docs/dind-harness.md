<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# DIND Harness (Deterministic Ironclad Nightmare Drills)

The DIND harness is the deterministic verification runner for Echo/WARP. It replays canonical intent transcripts and asserts that state hashes and intermediate outputs are identical across runs, platforms, and build profiles.

Location:
- `crates/echo-dind-harness`
- `crates/echo-dind-tests` (stable test app used by the harness)
- `testdata/dind` (scenarios + goldens)

## Quickstart

```bash
cargo run -p echo-dind-harness -- help
```

Examples (commands depend on the harness CLI):

```bash
cargo run -p echo-dind-harness -- torture
cargo run -p echo-dind-harness -- converge
cargo run -p echo-dind-harness -- repro <scenario>
```

## Determinism Guardrails

Echo ships guard scripts to enforce determinism in core crates:

- `scripts/ban-globals.sh`
- `scripts/ban-nondeterminism.sh`
- `scripts/ban-unordered-abi.sh`

## Convergence scope (Invariant B)

For commutative scenarios, `MANIFEST.json` can specify a `converge_scope`
node label (e.g., `sim/state`). The `converge` command compares the
projected hash of the subgraph reachable from that node, while still
printing full hashes for visibility.

### Converge scope semantics (short spec)

**What scopes exist today (DIND test app):**
- `sim/state` — the authoritative state root for the test app (includes theme/nav/route + kv).
- `sim/state/kv` (not currently used) — a narrower root for KV-only projections.

**What is included in the projected hash:**
- All nodes reachable by following **outbound edges** from the scope root.
- All edges where both endpoints are reachable.
- All node and edge attachments for the included nodes/edges.

**What is excluded:**
- Anything not reachable from the scope root (e.g., `sim/inbox`, event history, sequence sidecars).
- Inbound edges from outside the scope.

**What “commutative” means here:**
- The operations are order-independent with respect to the **projected subgraph**.
- Either they touch disjoint footprints or they are semantically commutative
  (e.g., set union on disjoint keys).

**When you must NOT use projection:**
- When event history is semantically meaningful (auditing, causality, timelines).
- When last-write-wins behavior or ordered effects are part of the contract.
- When differences in inbox/order should be observable by the consumer.

### CLI override (debug only)

`converge` accepts an override for ad‑hoc debugging:

```bash
cargo run -p echo-dind-harness -- converge --scope sim/state --i-know-what-im-doing <scenarios...>
```

This bypasses `MANIFEST.json` and emits a warning. Do not use it for canonical
test results.

Run them locally or wire them into CI for strict enforcement.
