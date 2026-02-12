<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Web Demo MVP

Priority: P1  
Status: Not Started  
Blocked By: none

Objective: deliver the first browser demo (WASM runtime + visualization), with Wesley as the schema/tooling supply chain.

## Features

- [F2.1 Wesley QIR Phase C](./F2.1-wesley-qir-phase-c.md) (Repo: Wesley)
- [F2.2 Wesley Migration Planning Phase B](./F2.2-wesley-migration-planning-phase-b.md) (Repo: Wesley)
- [F2.3 Wesley Go Public](./F2.3-wesley-go-public.md) (Repo: Wesley)
- [F2.4 echo-wesley-gen v2 Update](./F2.4-echo-wesley-gen-v2-update.md) (Repo: Echo)
- [F2.5 SHA-256 to BLAKE3 Coordination](./F2.5-sha256-to-blake3-coordination.md) (Repo: Shared)
- [F4.1 WASM Runtime Integration](./F4.1-wasm-runtime-integration.md) (Repo: Echo)
- [F4.2 In-Browser Visualization](./F4.2-in-browser-visualization.md) (Repo: Echo)
- [F4.3 echo-cas Browser Integration](./F4.3-echo-cas-browser-integration.md) (Repo: Echo)
- [F4.4 Wesley Type Pipeline in Browser](./F4.4-wesley-type-pipeline-in-browser.md) (Repo: Echo/Wesley)

## Exit Criteria

- Browser demo runs deterministically from a fixed seed.
- WASM build is reproducible and green in CI.
- View/state updates are visible and auditable through inspector hooks.
