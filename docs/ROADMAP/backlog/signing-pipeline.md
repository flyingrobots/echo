<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Signing Pipeline

> **Milestone:** [Backlog](README.md) | **Priority:** Unscheduled

CI and CLI support for signing and verifying release artifacts. Depends on the signing spec from F10.2.

**Issues:** #33, #34, #35, #36
**Chain:** #35 → #33 → #34 → #36

## T-10-3-1: Key Management Doc (#35)

**User Story:** As a release engineer, I want key management documentation so that I know how to generate, store, rotate, and revoke signing keys.

**Requirements:**

- R1: Document key generation procedure (algorithm, key size, tooling)
- R2: Document secure storage recommendations (hardware keys, CI secrets)
- R3: Document rotation procedure and timeline
- R4: Document revocation procedure and CRL/transparency-log approach

**Acceptance Criteria:**

- [ ] AC1: Document exists at `docs/KEY-MANAGEMENT.md`
- [ ] AC2: Covers generation, storage, rotation, and revocation
- [ ] AC3: Includes step-by-step commands for key generation
- [ ] AC4: Reviewed by at least one contributor

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** Documentation only. Key lifecycle procedures.
**Out of Scope:** Implementation of key management tooling, HSM integration.

**Test Plan:**

- **Goldens:** n/a (documentation)
- **Failures:** n/a
- **Edges:** n/a
- **Fuzz/Stress:** n/a

**Blocked By:** T-10-2-1
**Blocking:** T-10-3-2

**Est. Hours:** 3h
**Expected Complexity:** ~200 lines (markdown)

---

## T-10-3-2: CI — Sign Release Artifacts (Dry Run) (#33)

**User Story:** As a release engineer, I want CI to sign release artifacts automatically so that every release includes verifiable signatures without manual intervention.

**Requirements:**

- R1: Add a CI job that runs after build and produces detached signatures
- R2: Use a dummy/test key in dry-run mode (no production secrets yet)
- R3: Sign all `.wasm` and tarball artifacts
- R4: Upload signatures as CI artifacts alongside the binaries
- R5: Job must be idempotent (re-running produces identical signatures for identical inputs)

**Acceptance Criteria:**

- [ ] AC1: CI workflow includes a `sign-artifacts` job
- [ ] AC2: Dry-run mode works with a test key checked into the repo
- [ ] AC3: Signatures are uploaded as CI artifacts
- [ ] AC4: Job is idempotent — same input produces same signature

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** CI job (GitHub Actions), dry-run mode with test key.
**Out of Scope:** Production key integration, key rotation automation.

**Test Plan:**

- **Goldens:** Signature bytes for a known test artifact + test key
- **Failures:** Missing key file, corrupt artifact
- **Edges:** Empty artifact, very large artifact
- **Fuzz/Stress:** n/a

**Blocked By:** T-10-3-1
**Blocking:** T-10-3-3

**Est. Hours:** 4h
**Expected Complexity:** ~150 LoC (YAML + shell)

---

## T-10-3-3: CLI Verify Path (#34)

**User Story:** As a user, I want a CLI command to verify artifact signatures so that I can confirm artifacts are authentic before using them.

**Requirements:**

- R1: Add `echo verify <artifact> --sig <signature> --key <public-key>` CLI subcommand
- R2: Exit code 0 on success, non-zero on failure
- R3: Human-readable output indicating verification result
- R4: Support reading the public key from a file or environment variable

**Acceptance Criteria:**

- [ ] AC1: `echo verify` subcommand exists and is documented in `--help`
- [ ] AC2: Verification succeeds for a valid signature
- [ ] AC3: Verification fails with clear error for tampered artifact, wrong key, or corrupt signature
- [ ] AC4: Exit codes are correct (0 success, 1 failure, 2 usage error)

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** CLI subcommand for offline verification.
**Out of Scope:** Online key lookup, trust-on-first-use, key pinning.

**Test Plan:**

- **Goldens:** Verification output for known good artifact + signature
- **Failures:** Tampered artifact, wrong public key, truncated signature, missing file
- **Edges:** Artifact with zero bytes, signature file with trailing newline
- **Fuzz/Stress:** n/a

**Blocked By:** T-10-3-2
**Blocking:** T-10-3-4

**Est. Hours:** 4h
**Expected Complexity:** ~250 LoC

---

## T-10-3-4: CI — Verify Signatures (#36)

**User Story:** As a release engineer, I want CI to verify signatures of published artifacts so that any signing regression is caught automatically.

**Requirements:**

- R1: Add a CI job that downloads artifacts and their signatures, then verifies
- R2: Job runs after the signing job in the release pipeline
- R3: Verification failure fails the pipeline
- R4: Job logs which artifacts were verified and the result for each

**Acceptance Criteria:**

- [ ] AC1: CI workflow includes a `verify-signatures` job that depends on `sign-artifacts`
- [ ] AC2: Job verifies every signed artifact
- [ ] AC3: A tampered-artifact test case is included (CI should catch it)
- [ ] AC4: Verification results are logged per-artifact

**Definition of Done:**

- [ ] Code reviewed and merged
- [ ] Tests pass (CI green)
- [ ] Documentation updated (if applicable)

**Scope:** CI verification job, tamper-detection test.
**Out of Scope:** Production alerting, artifact registry integration.

**Test Plan:**

- **Goldens:** n/a
- **Failures:** Tampered artifact injected into CI (must fail the job)
- **Edges:** Artifact with no corresponding signature file
- **Fuzz/Stress:** n/a

**Blocked By:** T-10-3-3
**Blocking:** none

**Est. Hours:** 3h
**Expected Complexity:** ~100 LoC (YAML + shell)
