<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# MATH — Deterministic Math and Geometry

_Legend for making floating-point math and geometric primitives
identical across every platform Echo runs on._

## Goal

Eliminate every source of cross-platform float divergence and build
geometry primitives that are proven correct by property tests and
golden vectors.

This legend covers work like:

- IEEE 754 canonicalization (NaN payloads, subnormals, signed zero)
- deterministic trig oracle
- scalar types and canonical comparison
- collision detection and broad phase
- sweep-and-prune
- property testing for geometric primitives

## Human users

- James, designing deterministic physics and geometry
- game developers relying on cross-platform reproducibility
- contributors adding new geometric primitives or physics rules

## Agent users

- agents generating geometry tests or golden vectors
- agents validating float canonicalization claims against CI evidence
- agents writing property tests for edge cases

## Human hill

A human can add a new geometric operation and know — before merging —
whether it preserves cross-platform determinism, because the test
suite and golden vectors catch divergence automatically.

## Agent hill

An agent can inspect the deterministic math policy, the golden vectors,
and the property tests to programmatically verify that a proposed float
operation is safe under Echo's canonicalization rules.

## Core invariants

- Canonical NaN: all NaN payloads collapse to a single canonical
  representation.
- Flush subnormals to zero.
- Canonicalize signed zero to +0.0.
- No FMA unless explicitly opted in with documented precision
  guarantees.
- Golden vectors lock cross-platform output for trig and physics.

## Current cycle and backlog

- latest completed cycle: (none under METHOD yet)
- live backlog:
    - `asap/MATH_deterministic-trig.md`
    - `cool-ideas/MATH_tumble-tower-stage-0-aabb.md`
    - `cool-ideas/MATH_tumble-tower-stage-1-rotation.md`
    - `cool-ideas/MATH_tumble-tower-stage-2-friction.md`
    - `cool-ideas/MATH_tumble-tower-stage-3-sleeping.md`
