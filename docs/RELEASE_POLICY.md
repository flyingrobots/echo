<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Release Policy — TTD / Determinism Program

## Version

- Policy Version: 1.1
- Effective Date: 2026-02-15

## Gate Definitions

- **G1 Determinism**
    - Cross-platform parity for deterministic corpus (macOS + Linux; wasm checks as applicable).
    - Evidence: digest comparison artifact with run IDs and commit SHA.

- **G2 Decoder Security**
    - Negative tests prove rejection/handling of malformed payload classes.
    - Evidence: mapped test IDs + CI artifact output.

- **G3 Performance Regression Bound**
    - Benchmark delta for DET-critical hot paths within accepted threshold.
    - Evidence: baseline vs current perf artifact.

- **G4 Build Reproducibility**
    - Reproducible deterministic build constraints validated in CI.
    - Evidence: build artifact metadata and checksums.

## Blocker Matrix

The blocker matrix for release decisions:

```yaml
release_policy:
    staging_blockers: [G1, G2, G4]
    production_blockers: [G1, G2, G3, G4]
    # G3 is intentionally staging-optional: perf regressions are caught
    # before production but do not block functional validation in staging.
```

## Recommendation Rules

- **GO**: all required blockers are VERIFIED.
- **CONDITIONAL**: one or more required blockers are UNVERIFIED/INFERRED with approved closeout plan.
- **NO-GO**: required blocker FAILED or unresolved with no approved mitigation.

## Gate States

- **VERIFIED**: Evidence exists in the form of immutable CI artifacts (run ID, commit SHA) proving the gate pass.
- **INFERRED**: High confidence that the gate passes based on circumstantial evidence (e.g., downstream tests pass), but direct artifact-backed proof is pending.
- **UNVERIFIED**: No supporting evidence currently exists.

## Closeout Plan

An **Approved Closeout Plan** is required for any CONDITIONAL release.

- **Definition**: A documented set of tasks, owners, and ETAs to move a gate from UNVERIFIED/INFERRED to VERIFIED.
- **Approval Authority**: Must be approved by the **Architect** or **Security Engineer** role as defined in `det-policy.yaml` for the affected crate.

## Evidence Rules

A gate may be marked VERIFIED only with immutable pointers:

- workflow/job name
- run ID
- commit SHA
- artifact filename
- checksum (where relevant)

No immutable evidence => gate must be INFERRED or UNVERIFIED.

## Escalation

If staging/prod blocker state conflicts with recommendation:

1. Freeze recommendation to CONDITIONAL.
2. Open blocker issues with owners and ETA.
3. Re-run gate suite before release decision.
