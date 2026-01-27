<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# SPEC-0005-CAS

> Ref-First Content Addressed Storage (CAS) v1

This spec is split into smaller documents for readability:

1. [SPEC-0005-CAS-01-Foundations](SPEC-0005-CAS-01-Foundations.md)
    - Axioms, core types, canonical encoding, typed refs, wire protocol, schema distribution,
      validity rules, conformance tests, and rollout order.
2. [SPEC-0005-CAS-02-Worldlines](SPEC-0005-CAS-02-Worldlines.md)
    - CAS integration with worldlines/TTDR, retention/GC roots, migration plan, and recommendations.
3. [SPEC-0005-CAS-03-Wire-V1](SPEC-0005-CAS-03-Wire-V1.md)
    - CAS wire v1 layout, message formats, versioning, and struct-level guidance.
4. [SPEC-0005-CAS-04-Rust-Reference](SPEC-0005-CAS-04-Rust-Reference.md)
    - Rust reference implementation sketches (drop-in modules).
5. [SPEC-0005-CAS-05-Session-Envelope](SPEC-0005-CAS-05-Session-Envelope.md)
    - CAS session envelope integration spec (OpEnvelope/JIT!).

Each part is intended to be ~500 LOC or less.
