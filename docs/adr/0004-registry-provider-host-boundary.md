<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR 0004: Registry, Provider, and Host Boundary

- **Status:** Accepted
- **Date:** 2026-07-13

## Context

Generated artifacts, runtime registration, host-provided executors, and
application dispatch have different authority. Combining them into one API
would grant application code runtime-owner capabilities.

## Decision

Compilers emit verified package artifacts and registration descriptors. Echo
verifies and registers packages into a runtime-local registry. Trusted hosts
supply only the executors, observers, capabilities, and runtime controls they
are authorized to provide. Applications receive product-facing submission and
observation adapters; they do not receive package-install, ingress-staging,
scheduler, fault-recovery, or WAL authority.

Registration must preflight all package, schema, artifact, codec, operation,
query, and compatibility mappings before mutating engine state.

## Consequences

- Compiler identity is evidence, not runtime authority.
- A trusted host may install and run generated packages without exposing Echo
  coordinates to the application.
- Application nouns remain in contracts and adapters outside Echo core.

## Evidence Anchors

- `docs/architecture/application-contract-hosting.md`
- `crates/warp-core/src/contract_registry.rs`
- `crates/warp-core/src/trusted_runtime_host.rs`
- `crates/echo-wesley-gen`
