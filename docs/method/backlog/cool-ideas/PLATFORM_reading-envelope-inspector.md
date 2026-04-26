<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Reading envelope inspector

Status: active cool idea. `ReadingEnvelope` and related observer-plan,
budget, rights, witness, and residual posture fields exist in
`echo-wasm-abi`, and `warp-wasm` emits reading envelopes in observation
artifacts. No local inspector/debug view renders that structure yet.
This card remains operational because it turns the active
reading-envelope boundary into a maintainer-facing inspection surface.

This is a local-first inspection surface for making the read-side doctrine
visible instead of merely asserted.

## Why now

The stack keeps saying:

- observation is not just snapshot
- a reading should carry witness, coordinate, and posture

An inspector would make that concrete for maintainers and debugger/tool
consumers. It would help prove that read-side outputs are richer than a naked
payload without forcing every consumer to invent its own debug wrapper.

## Hill

A maintainer can inspect one reading envelope and immediately see:

- plan identity
- coordinate/frontier
- payload
- witness reference
- rights and budget posture
- plurality or obstruction posture

## Done looks like

- one local surface or debug view renders reading-envelope structure clearly
- the surface is useful for Echo runtime debugging and downstream consumer work
- the inspector helps validate the eventual shared family boundary instead of
  encouraging ad hoc local wrappers

## Repo evidence

- `docs/method/backlog/asap/PLATFORM_observer-plan-reading-artifacts.md`
- `docs/method/backlog/up-next/PLATFORM_reading-envelope-family-boundary.md`
