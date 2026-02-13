<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Security

> **Milestone:** [Backlog](README.md) | **Priority:** Unscheduled

Specifications and hardening for trust boundaries across FFI, WASM, and CLI surfaces. Includes commit signing specs, security context definitions, FFI validation, packet checksums, and provenance envelopes.

**Issues:** #20, #21, #38, #195, #202

## T-10-2-1: Spec — Commit/Manifest Signing (#20)

**User Story:** As a deployment operator, I want a specification for signing commits and manifests so that I can verify the integrity and authorship of simulation artifacts.

**Requirements:**

- R1: Define what gets signed (commit hash, manifest digest, or both)
- R2: Specify signature format (detached Ed25519 or similar)
- R3: Define the canonical byte sequence that is signed (no ambiguity)
- R4: Specify how signatures are stored alongside artifacts
- R5: Address key rotation and revocation at the spec level

**Acceptance Criteria:**

- [ ] AC1: Spec document exists at `docs/specs/SPEC-SIGNING.md`
- [ ] AC2: Canonical signing input is defined unambiguously
- [ ] AC3: Spec covers key lifecycle (generation, rotation, revocation)
- [ ] AC4: Spec reviewed by at least one contributor

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Specification only. Covers commits and manifests.
**Out of Scope:** Implementation, CI integration, key management tooling.

**Test Plan:**

- **Goldens:** n/a (spec document)
- **Failures:** n/a
- **Edges:** n/a
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** T-10-3-1

**Est. Hours:** 4h
**Expected Complexity:** ~250 lines (markdown)

---

## T-10-2-2: Spec — Security Contexts (#21)

**User Story:** As a runtime integrator, I want clearly defined security contexts for FFI, WASM, and CLI boundaries so that I understand what each boundary permits and denies.

**Requirements:**

- R1: Enumerate all trust boundaries in the Echo architecture (FFI, WASM, CLI, network)
- R2: For each boundary, define allowed operations and data flow direction
- R3: Define the threat model (what adversary capabilities are assumed)
- R4: Specify how sandboxing is enforced in WASM vs. native builds

**Acceptance Criteria:**

- [ ] AC1: Spec document exists at `docs/specs/SPEC-SECURITY-CONTEXTS.md`
- [ ] AC2: All four trust boundaries are enumerated with allowed operations
- [ ] AC3: Threat model section is explicit about assumptions
- [ ] AC4: Spec reviewed by at least one contributor

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Specification only. All runtime trust boundaries.
**Out of Scope:** Implementation of sandboxing, policy enforcement code.

**Test Plan:**

- **Goldens:** n/a (spec document)
- **Failures:** n/a
- **Edges:** n/a
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** T-10-2-3

**Est. Hours:** 4h
**Expected Complexity:** ~300 lines (markdown)

---

## T-10-2-3: FFI Limits and Validation (#38)

**User Story:** As the Echo runtime, I want input validation at every FFI boundary so that malformed or malicious inputs cannot cause undefined behavior or panics.

**Requirements:**

- R1: Audit all existing FFI entry points and catalog their input types
- R2: Add length/range checks for all buffer and numeric inputs
- R3: Return typed errors (not panics) for all validation failures
- R4: Add `#[deny(unsafe_op_in_unsafe_fn)]` to all FFI modules
- R5: Document the validation contract for each entry point

**Acceptance Criteria:**

- [ ] AC1: Every FFI entry point has explicit input validation
- [ ] AC2: No FFI function panics on malformed input (returns error instead)
- [ ] AC3: At least one test per FFI function exercises the rejection path
- [ ] AC4: `unsafe` audit annotations are present

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** All FFI entry points in the echo crate.
**Out of Scope:** WASM boundary validation (separate task), network input validation.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** Zero-length buffer, oversized buffer, null pointer, out-of-range enum discriminant
- **Edges:** Exactly-at-limit inputs, empty strings, max u64 values
- **Fuzz/Stress:** Property-based tests with `proptest` for buffer inputs

**Blocked By:** T-10-2-2
**Blocking:** none

**Est. Hours:** 6h
**Expected Complexity:** ~350 LoC

---

## T-10-2-4: JS-ABI Packet Checksum v2 (#195)

**User Story:** As a JS-ABI consumer, I want packet checksums to use domain-separated hashing so that checksum collisions across different packet types are cryptographically impossible.

**Requirements:**

- R1: Define domain separation prefixes for each packet type (e.g., `"echo.tick.v1"`, `"echo.state.v1"`)
- R2: Replace existing checksum with `SHA-256(domain_prefix || length || payload)`
- R3: Maintain backward compatibility: accept both old and new checksums during a transition window
- R4: Emit a deprecation warning when an old-format checksum is received

**Acceptance Criteria:**

- [ ] AC1: All packet types have assigned domain prefixes
- [ ] AC2: New checksum computation uses domain-separated hashing
- [ ] AC3: Old checksums are accepted with a logged deprecation warning
- [ ] AC4: Round-trip test: JS → WASM → JS with new checksum format

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Checksum computation and validation for all JS-ABI packet types.
**Out of Scope:** Migrating to BLAKE3, removing old checksum support (future task).

**Test Plan:**

- **Goldens:** Bit-exact checksum values for known packets
- **Failures:** Truncated packet, wrong domain prefix, corrupted checksum
- **Edges:** Empty payload, maximum-size payload
- **Fuzz/Stress:** Fuzz packet payloads and verify checksum round-trip

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 5h
**Expected Complexity:** ~250 LoC

---

## T-10-2-5: Spec — Provenance Payload v1 (#202)

**User Story:** As an auditor, I want a canonical envelope format for artifact provenance so that I can trace the full lineage and verify signatures of any simulation artifact.

**Requirements:**

- R1: Define the Provenance Payload (PP) envelope structure (header, claims, signatures)
- R2: Specify the canonical serialization (deterministic JSON or CBOR)
- R3: Define claim types: `built-by`, `derived-from`, `signed-by`, `reviewed-by`
- R4: Specify how PPs chain (each PP can reference parent PPs by hash)
- R5: Align with in-toto or SLSA attestation formats where practical

**Acceptance Criteria:**

- [ ] AC1: Spec document exists at `docs/specs/SPEC-PROVENANCE-PAYLOAD.md`
- [ ] AC2: Envelope structure is fully defined with field-level documentation
- [ ] AC3: At least two worked examples (single artifact, chained artifacts)
- [ ] AC4: Relationship to SLSA levels is explicitly discussed

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Specification only. Envelope format and claim vocabulary.
**Out of Scope:** Implementation, storage, query API, trust policy engine.

**Test Plan:**

- **Goldens:** n/a (spec document)
- **Failures:** n/a
- **Edges:** n/a
- **Fuzz/Stress:** n/a

**Blocked By:** none
**Blocking:** none

**Est. Hours:** 4h
**Expected Complexity:** ~350 lines (markdown)
