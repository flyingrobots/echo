<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Guide — Echo

This is the developer-level operator guide for Echo. Use it for orientation and
the productive-fast path through Echo's deterministic WARP runtime over
witnessed causal history.

For deep-track doctrine, theoretical foundations (AION Foundations), and internal spec details, use [ADVANCED_GUIDE.md](./ADVANCED_GUIDE.md).

## Choose Your Lane

### 1. Host a Contract Package

Exercise the local generated-style package boundary. Product behavior belongs
in authored, generated contracts; the current generator limitations are stated
explicitly in the generated-rule topic.

- **Read**: [Local Contract Host Quickstart](./docs/quickstart-local-contract-host.md)
- **Understand**: [Application Contract Hosting](./docs/architecture/application-contract-hosting.md)
- **Check status**: [Generated Rule Authorship](./docs/topics/GeneratedRules.md)

### 2. Verify Determinism (DIND)

Use the "Drill Sergeant" discipline to prove cross-platform convergence.

- **Read**: [DIND Harness](./docs/determinism/dind-harness.md)
- **Run**: `cargo xtask dind run`

### 3. Continuous Integration

Understand the guardrails that prevent non-determinism from entering main.

- **Check**: [`det-policy.yaml`](./det-policy.yaml)
- **Scripts**: `scripts/ban-nondeterminism.sh`

## Big Picture

There is witnessed causal history. WARP optics chart it. Holograms witness
those charts. Materialized graphs are optional readings. Continuum is the
protocol for lawful causal-history exchange.

External callers submit explicit-base intents or observe through bounded
optics. Echo admits, stages, pluralizes, conflicts, or obstructs those claims
under named law and emits receipts, readings, or witnesses. Application nouns
stay in authored contracts and generated adapters rather than Echo core.

## Orientation Checklist

- [ ] **I am setting up the repo**: Run `make hooks` and `cargo check`.
- [ ] **I am adding product behavior**: Author it in the contract language and use generated adapters; raw Rust rule registration is bootstrap-only.
- [ ] **I am debugging a desync**: Run `cargo xtask dind run --emit-repro` to emit a reproduction bundle on failure.
- [ ] **I am contributing to Echo**: Read [AGENTS.md](./AGENTS.md), then inspect the relevant GitHub issue or pull request.

## Rule of Thumb

If you need a comprehensive spec, use the [docs/README.md](./docs/README.md) map.

If you need current architectural truth, use the
[architecture](./docs/architecture/), [specification](./docs/spec/),
[invariant](./docs/invariants/), and [topic](./docs/topics/) maps. Durable
decisions live in [ADRs](./docs/adr/); live work and status live in GitHub.

If you are just starting, use the [README.md](./README.md) and the orientation tracks above.

## Focused Durability Witnesses

Use the narrow repository-owned slices while iterating on WAL and recovery:

```bash
cargo xtask test-slice runtime-wal-ack
cargo xtask test-slice durable-runtime-wal
cargo xtask test-slice durability-release
```

---

**The goal is inevitability. Every state transition is a provable consequence of its causal history.**
