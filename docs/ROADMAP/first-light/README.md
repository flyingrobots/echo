<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# First Light

> **Priority:** P1 | **Status:** Not Started | **Est:** ~88h

The crown jewel — TTD (Tick-based Deterministic engine) running in-browser. Every user interaction is a graph rewrite, rendered live. This milestone includes the Wesley pipeline work that feeds the website, the WASM runtime integration, browser visualization, echo-cas browser validation, and Wesley type bridging across JS/WASM.

**Blocked By:** —

## Exit Criteria

- [ ] Browser demo runs deterministically from a fixed seed
- [ ] WASM build reproducible in CI
- [ ] Render + state sync observable in inspector hooks
- [ ] Wesley-generated types cross JS/WASM boundary without manual glue
- [ ] echo-cas MemoryTier validated under WASM

## Features

| Feature                         | File                                                               | Repo   | Est. | Status      |
| ------------------------------- | ------------------------------------------------------------------ | ------ | ---- | ----------- |
| Wesley QIR Phase C              | [wesley-qir-phase-c.md](wesley-qir-phase-c.md)                     | Wesley | ~12h | Not Started |
| Wesley Migration Planning       | [wesley-migration.md](wesley-migration.md)                         | Wesley | ~10h | Not Started |
| Wesley Go Public                | [wesley-go-public.md](wesley-go-public.md)                         | Wesley | ~6h  | Not Started |
| echo-wesley-gen v2 Update       | [echo-wesley-gen-v2.md](echo-wesley-gen-v2.md)                     | Echo   | ~5h  | Not Started |
| SHA-256 to BLAKE3 Coordination  | [sha256-blake3.md](sha256-blake3.md)                               | Shared | ~4h  | Not Started |
| WASM Runtime Integration        | [wasm-runtime.md](wasm-runtime.md)                                 | Echo   | ~16h | Not Started |
| In-Browser Visualization        | [browser-visualization.md](browser-visualization.md)               | Echo   | ~15h | Not Started |
| echo-cas Browser Integration    | [echo-cas-browser.md](echo-cas-browser.md)                         | Echo   | ~7h  | Not Started |
| Wesley Type Pipeline in Browser | [wesley-type-pipeline-browser.md](wesley-type-pipeline-browser.md) | Shared | ~13h | Not Started |
