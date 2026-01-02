<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
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
ECS is a *useful storage and API layer*, but the deeper “ground truth” model is the graph rewrite system.

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

1. WARP View Protocol demo: [/guide/wvp-demo](/guide/wvp-demo)
2. Collision tour: [/guide/collision-tour](/guide/collision-tour)
3. Interactive collision DPO tour (static HTML): [/collision-dpo-tour.html](/collision-dpo-tour.html)

### If you want “what should I work on?”

- Execution plan (living intent, “Today’s Intent” at the top): [/execution-plan](/execution-plan)
- Decision log (chronological record of decisions): [/decision-log](/decision-log)
- Docs map (curated index): [/docs-index](/docs-index)

## How These Docs Are Organized

- **Guides** (`docs/guide/`): newcomer-friendly explanations and runnable walkthroughs.
- **Specs** (`docs/spec-*.md`, `docs/spec/`): normative artifacts we try to keep stable and precise.
- **Notes** (`docs/notes/`): explorations and scratchpads; useful, but not authoritative.
- **Book** (`docs/book/`): long-form LaTeX material; may lag behind the latest implementation.

## Viewing Docs Locally

From the repo root:

- Install dependencies: `pnpm install`
- Run the dev server: `pnpm docs:dev`
- Build (link checks / CI gate): `pnpm docs:build`

The dev server prints a local URL (typically `http://localhost:5173`).
