<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# CI det-policy hardening

Harden the determinism classification CI pipeline.

- Unit tests for `classify_changes.cjs` and `matches()` (#286)
- Auto-generate `DETERMINISM_PATHS` from `det-policy.yaml` DET_CRITICAL
  entries (#285)
- Per-crate gate overrides in det-policy classification system (#284)

These three are tightly coupled — they all protect the determinism
guarantee at the CI level.
