<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Proof Core

> **Priority:** P1 | **Status:** Planned | **Est:** ~18h

Cross-OS determinism proof and trig oracle verification. The deliverable is _Determinism Claims v0.1 (Scope + Evidence + Limits)_.

**Blocked By:** Lock the Hashes

## Exit Criteria

- [ ] 1-thread vs N-thread determinism harness green across {macOS, Linux}
- [ ] Deterministic trig oracle verified against reference values
- [ ] "Determinism Claims v0.1" document published (scope + evidence + limits)
- [ ] Repro script produces identical receipts/checksums over 100 reruns

## Features

| Feature                     | File                                             | Est. | Status      |
| --------------------------- | ------------------------------------------------ | ---- | ----------- |
| Determinism Torture Harness | [determinism-torture.md](determinism-torture.md) | ~10h | Not Started |
| Deterministic Trig Oracle   | [deterministic-trig.md](deterministic-trig.md)   | ~4h  | Not Started |
| Docs Polish                 | [docs-polish.md](docs-polish.md)                 | ~4h  | Not Started |
