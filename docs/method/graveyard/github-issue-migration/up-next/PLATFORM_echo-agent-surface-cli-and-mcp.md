---
audit-date: 2026-06-15
audit-status: keep
topics:
    - mcp
    - cli
    - agent
accuracy: 95%
issue: 487
findings:
    - claim: "Echo lacks an inspectable Model Context Protocol (MCP) server surface exposing echo nouns to AI agents"
      ruling: true
      evidence:
          - filepath: "crates/warp-cli/README.md"
            line: 1
            git-sha: "5f85dae5727d36acf4a82aad8d7cdb0488cb67be"
---

<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Add an explicit Echo CLI and MCP agent surface

Echo is browser-hostable and increasingly Continuum-aligned, but it is still
not agent-native in the METHOD sense.

Today the repo has:

- runtime truth in Rust types and runtime schemas
- a browser-hostable WASM runtime boundary through `warp-wasm`
- rich local runtime objects for observation, playback, provenance, and
  scheduler inspection

What it does **not** have is one explicit, inspectable agent boundary such as:

- a narrow CLI for observation, playback, neighborhood, and receipt inspection
- an MCP surface exposing the same nouns and controls to tools/agents

That gap matters for at least three reasons:

1. it makes agent use depend on local Rust APIs or ad hoc browser bridges
2. it weakens the shared Echo / `warp-ttd` / Continuum integration story
3. it keeps Echo behind METHOD's "agent surface first" direction even when the
   underlying runtime truth is already strong

This project should answer:

1. what the minimum Echo CLI surface is
2. whether MCP should sit directly over runtime objects or over the CLI/session
   vocabulary
3. which nouns must be shared with Continuum / `warp-ttd`, and which remain
   Echo-local shell
4. how this agent surface relates to, but does not replace, the browser host
   bridge

The first honest target is not a giant tool catalog. It is one narrow,
inspectable surface for:

- host/runtime identity
- coordinates and lane identity
- observation / playback frame reads
- neighborhood core and reintegration detail once published
- receipt shell and scheduler/runtime shell
- step / seek / mode transitions where Echo actually supports them

Related:

- Echo design `0005-echo-ttd-witness-surface`
- Echo design `0006-echo-continuum-alignment`
- Echo WASM ABI docs in `docs/spec/SPEC-0009-wasm-abi.md`
- `warp-ttd` docs `CLI.md`, `MCP.md`, and `BEARING.md`
