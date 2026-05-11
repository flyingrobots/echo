<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# KERNEL - Admission Outcome Family

Echo currently has outcome-shaped truth spread across tick receipts, settlement
reason classes, and braid/collapse language, but it still lacks one explicit
admission outcome family that the runtime, docs, and future shared publication
surface can all point to.

This cycle should define and thread one lawful outcome algebra through Echo:

- `Derived`
- `Plural`
- `Conflict`
- `Obstruction`

The first cut does not need to rewire every subsystem. It does need to stop the
runtime from speaking different dialects about the same causal fact.
