<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0003 - DPO Concurrency Litmus v0

_Pin the executable concurrency claim Echo can make today: footprint-independent rewrites settle to a deterministic outcome._

Legend: KERNEL

Depends on:

- [WARP Rewrite Scheduler](scheduler-warp-core.md)
- [WARP Tick Patch](warp-tick-patch.md)
- [Merkle Commit](merkle-commit.md)

## Why this packet exists

The research language points toward DPO/DPOI concurrency: independent rewrites commute, conflicts have a deterministic explanation, and a witnessed history can be replayed. The current runtime does not implement a full categorical DPO engine. It implements a conservative footprint scheduler.

## Human users / jobs / hills

Human users need determinism claims that are reviewable rather than aspirational.

The hill: a reviewer can run the litmus tests and see enqueue-order permutations produce the same terminal commit when the footprints admit the same independent set.

## Agent users / jobs / hills

Agent users need a concrete translation between theory vocabulary and runtime evidence.

The hill: an agent can inspect a rejected candidate and identify the footprint resource that blocked it.

## Decision 1: v0 speaks in footprint independence

Given the same starting state and candidate set, the engine selects and applies a deterministic admissible subset using canonical ordering and footprint conflict checks. This is not a proof of full DPO concurrency. It is the runtime's executable settlement rule.

## Decision 2: Critical-pair pressure appears as footprint conflict

Two candidates conflict when a declared write overlaps another candidate's read or write, or when their boundary port claims overlap. The losing candidate is rejected with `FootprintConflict` in the tick receipt.

## Decision 3: Deterministic order is part of the witness

Candidates drain in canonical order derived from `scope_hash`, stable rule id, and a nonce tie-breaker. The nonce is not a semantic clock.

## Decision 4: No hidden dependencies

The litmus claim only holds when footprints soundly over-approximate rule effects. A rule that reads or writes undeclared state is a kernel bug.

## Litmus families

Commuting pair: disjoint footprints, both admitted, terminal commit identical across enqueue permutations.

Critical-pair-style overlap: conflicting footprints, deterministic winner, deterministic rejection.

Shared scope with separable resources: high-level target may be the same, but declared resources do not conflict, so both can settle.

## Evidence

Implementation evidence lives in `crates/warp-core/src/scheduler.rs`, `crates/warp-core/src/footprint.rs`, and `crates/warp-core/tests/dpo_concurrency_litmus.rs`.
