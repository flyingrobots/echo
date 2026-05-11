<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# KERNEL - Admission Outcome Family

Echo now has an explicit `AdmissionOutcomeKind` family, but outcome-shaped truth
still needs to be threaded consistently across tick receipts, settlement reason
classes, and braid/collapse language so the runtime, docs, and future shared
publication surface all point to the same causal vocabulary.

This cycle should define and thread one lawful outcome algebra through Echo:

- `Derived`
- `Plural`
- `Conflict`
- `Obstruction`

The remaining work does not need to rewire every subsystem at once. It does
need to finish removing older dialects so `Derived`, `Plural`, `Conflict`, and
`Obstruction` remain the shared causal fact family.
