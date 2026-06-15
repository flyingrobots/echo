---
audit-date: 2026-06-15
audit-status: archive
topics:
    - admission
    - outcome
    - algebra
accuracy: 100%
issue: 468
findings:
    - claim: "The AdmissionOutcomeKind fact family (Admitted, Staged, Plural, Conflict, Obstructed) is fully implemented"
      ruling: true
      evidence:
          filepath: "crates/warp-core/src/admission.rs"
          line: 178
          git-sha: "5f85dae5727d36acf4a82aad8d7cdb0488cb67be"
---

<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# KERNEL - Admission Outcome Family

Echo now has an explicit `AdmissionOutcomeKind` family, but outcome-shaped truth
still needs to be threaded consistently across tick receipts, settlement reason
classes, and braid/collapse language so the runtime, docs, and future shared
publication surface all point to the same causal vocabulary.

This cycle should finish threading one lawful witnessed-suffix outcome algebra
through Echo:

- `Admitted`
- `Staged`
- `Plural`
- `Conflict`
- `Obstructed`

The remaining work does not need to rewire every subsystem at once. It does
need to finish removing older dialects so `Admitted`, `Staged`, `Plural`,
`Conflict`, and `Obstructed` remain the shared causal fact family.
