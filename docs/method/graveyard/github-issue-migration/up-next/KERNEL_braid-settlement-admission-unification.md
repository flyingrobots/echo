---
audit-date: 2026-06-15
audit-status: archive
topics:
    - braid
    - settlement
    - admission
accuracy: 100%
issue: 470
findings:
    - claim: "Braid comparison and strand settlement have been fully unified under the common AdmissionOutcomeKind algebra"
      ruling: true
      evidence:
          - filepath: "crates/warp-core/src/settlement.rs"
            line: 253
            git-sha: "5f85dae5727d36acf4a82aad8d7cdb0488cb67be"
---

<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# KERNEL - Braid and Settlement Admission Unification

Strands now obey the one-`super_tick()` law in doctrine and runtime shape, but
braid and settlement still risk feeling like side corridors instead of further
instances of the same admission architecture.

This cycle should push Echo toward one honest story:

- braid comparison is a plural object over a common basis
- collapse or settlement is an admission act over that plural object
- lawful outcomes remain `Derived`, `Plural`, `Conflict`, or `Obstruction`
- publication artefacts stay distinct from witness cores

The first cut should narrow the semantic gap, not solve the entire final braid
ontology.
