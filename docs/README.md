<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Docs

Echo's documentation describes a deterministic WARP runtime over witnessed
causal history. Git history is the archive; GitHub owns live work and status.

## Start Here

- [Architecture outline](architecture/outline.md)
- [Application contract hosting](architecture/application-contract-hosting.md)
- [Local contract host quickstart](quickstart-local-contract-host.md)
- [Echo 1.0 release contract](releases/echo-1.0-contract.md)
- [WARP core runtime](spec/warp-core.md)

## Current Architecture

- [There Is No Graph](architecture/there-is-no-graph.md)
- [Continuum foundations](architecture/continuum-foundations.md)
- [Continuum transport](architecture/continuum-transport.md)
- [WSC, Verkle, IPA, and retained readings](architecture/wsc-verkle-ipa-retained-readings.md)
- [Echo optics adapter notes](architecture/echo-optics-adapter-notes.md)

## Living Topics

- [Topic map](topics/README.md)
- [Runtime authority](topics/RuntimeAuthority.md)
- [Generated rules](topics/GeneratedRules.md)
- [WARP optics](topics/WarpOptics.md)
- [Strands and braids](topics/StrandsAndBraids.md)
- [Obstructions](topics/Obstructions.md)
- [Causal anchors](topics/CausalAnchors.md)
- [WAL](topics/WAL.md)
- [Runtime constellation](topics/RuntimeConstellation.md)

## Durable Decisions

- [ADR map](adr/README.md)
- [Repository knowledge model](adr/0001-repository-knowledge-model.md)
- [Echo/Continuum authority boundary](adr/0002-echo-continuum-authority-boundary.md)
- [Generated rule authorship and footprints](adr/0003-generated-rule-authorship-and-footprints.md)
- [Registry, provider, and host boundary](adr/0004-registry-provider-host-boundary.md)
- [Continuum transport identity](adr/0005-continuum-transport-identity.md)
- [Universal little-endian codec](adr/0006-universal-little-endian-codec.md)
- [Session causal posture and authority](adr/0007-sessions-causal-posture-and-authority.md)
- [Bunny owns reusable geometry](adr/0008-bunny-owns-reusable-geometry.md)

## Normative Contracts

- [WASM ABI](spec/SPEC-0009-wasm-abi.md)
- [JS-to-CBOR mapping](spec/js-cbor-mapping.md)
- [ABI golden vectors](spec/abi-golden-vectors.md)
- [Two-plane law](invariants/warp-two-plane-law.md)
- [Strand contract](invariants/STRAND-CONTRACT.md)
- [Fixed timestep](invariants/FIXED-TIMESTEP.md)
- [Declarative rule authorship](invariants/DECLARATIVE-RULE-AUTHORSHIP.md)

## Determinism Evidence

- [Deterministic math policy](determinism/SPEC_DETERMINISTIC_MATH.md)
- [Deterministic math hazards](determinism/DETERMINISTIC_MATH.md)
- [Claim register](determinism/DETERMINISM_CLAIMS_v0.1.md)
- [DIND harness](determinism/dind-harness.md)
- [Release policy](determinism/RELEASE_POLICY.md)
- [Scheduler performance](benchmarks/scheduler-performance-warp-core.md)

## Knowledge Ownership

Current architecture belongs in architecture documents, specifications,
invariants, and living topics. Accepted durable decisions belong in ADRs.
Externally meaningful shipped behavior belongs in `CHANGELOG.md`. Live design,
priority, dependencies, review state, and follow-up work belong in GitHub
Issues, Projects, pull requests, and review threads.
