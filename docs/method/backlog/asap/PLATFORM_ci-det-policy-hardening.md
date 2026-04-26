<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# CI det-policy hardening

Harden the determinism classification CI pipeline.

Status: active and partially implemented.

- Unit tests for `classify_changes.cjs` and `matches()` (#286, open)
- Auto-generate `DETERMINISM_PATHS` from `det-policy.yaml` DET_CRITICAL
  entries (#285, completed in `.github/workflows/det-gates.yml`)
- Per-crate gate overrides in det-policy classification system (#284, open)

The remaining open pieces are still tightly coupled: script tests make the
classifier safe to change, and per-crate overrides are the next precision step
after the broad `scripts/**` fail-safe.
