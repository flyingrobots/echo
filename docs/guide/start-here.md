<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Start Here

This page is the “front door” for Echo. If you only read one guide before diving into specs or code,
read this.

## What Echo Is

Echo is a deterministic simulation core built around **WARP**:

- **State is a graph** (structure) plus **attachments** (data).
- A tick is a deterministic set of **graph rewrites**.
- Determinism is treated as a first-class feature: replay, hashing, slicing, and sync are design inputs.

If you come in expecting a traditional ECS, it helps to reframe:
ECS is a _useful storage and API layer_, but the deeper “ground truth” model is the graph rewrite system.

## Recommended Reading Paths

### If you’re not a programmer (or don’t want code yet)

1. Echo, explained like you’re not a programmer: [/guide/eli5](/guide/eli5)
2. Start Here (this page): [/guide/start-here](/guide/start-here)
3. WARP primer (newcomer-friendly, but more precise): [/guide/warp-primer](/guide/warp-primer)

### If you’re new to WARP / graph-rewrite simulation

1. WARP primer: [/guide/warp-primer](/guide/warp-primer)
2. Two-plane law (why “no hidden edges” exists): [/warp-two-plane-law](/warp-two-plane-law)
3. Core runtime spec (`warp-core`): [/spec-warp-core](/spec-warp-core)

### If you want to run something end-to-end

1. Collision DPO tour (static HTML): [/collision-dpo-tour.html](/collision-dpo-tour.html)

Echo no longer ships the older local WVP demo stack. Browser debugger delivery
is moving to `warp-ttd`, while Echo keeps the WASM/browser host surfaces.

### Collision DPO Tour (what to expect)

- The tour shows collision + CCD as **graph rewrites** and lets you step rule-by-rule.
- Use the World/Graph tabs in the picture-in-picture panel to compare model vs visualization.
- Prev/Next steps you through each rewrite; think “proof by inspection,” not just animation.

### If you want what should I work on?

- Docs home / curated map: [/](/)

## How These Docs Are Organized

- **Guides** (`docs/guide/`): newcomer-friendly explanations and runnable walkthroughs.
- **Specs** (`docs/spec-*.md`, `docs/spec/`): normative artifacts we try to keep stable and precise.
- **Architecture / Theory** (`docs/architecture/`, `docs/theory/`): design intent and conceptual framing.
- **Procedures / Benchmarks** (`docs/procedures/`, `docs/benchmarks/`): contributor workflow and evidence.

## Viewing Docs Locally

From the repo root:

- Install dependencies: `pnpm install`
- Run the dev server: `pnpm docs:dev`
- Build (link checks / CI gate): `pnpm docs:build`

The dev server prints a local URL (typically `http://localhost:5173`).
