<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Proof Core

> **Priority:** P1 | **Status:** In Progress | **Est:** ~18h
> **Evidence:** `docs/determinism/DETERMINISM_CLAIMS_v0.1.md`, `testdata/trig_golden_2048.bin`

Cross-OS determinism proof and trig oracle verification. The deliverable is _Determinism Claims v0.1 (Scope + Evidence + Limits)_.

**Blocked By:** Lock the Hashes ✅, Developer CLI ✅

## Exit Criteria

- [x] 1-thread vs N-thread determinism harness green across {macOS, Linux}
- [x] Deterministic trig oracle verified against reference values
- [x] "Determinism Claims v0.1" document published (scope + evidence + limits)
- [x] Repro script produces identical receipts/checksums over 100 reruns

## Features

| Feature                     | File                                             | Est. | Status      |
| --------------------------- | ------------------------------------------------ | ---- | ----------- |
| Determinism Torture Harness | [determinism-torture.md](determinism-torture.md) | ~10h | Verified    |
| Deterministic Trig Oracle   | [deterministic-trig.md](deterministic-trig.md)   | ~4h  | Verified    |
| Docs Polish                 | [docs-polish.md](docs-polish.md)                 | ~4h  | In Progress |
