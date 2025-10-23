# Echo Engine

Echo is a deterministic, renderer-agnostic game engine spine forged from the ashes of the 2013 Caverns prototype. Its mission: treat **time** as a first-class playground—branchable timelines, mergeable realities, and tooling that makes temporal gymnastics feel routine.

## Why Echo?
- **Deterministic Core** – fixed-step scheduler, archetype storage, and reproducible math make rollback, replay, and multiverse trickery possible.
- **Hexagonal Architecture** – input, rendering, physics, audio, networking, and persistence live behind ports so adapters can range from Pixi to WebGPU to TUIs.
- **Temporal Sandbox** – Codex’s Baby (event bus) and a persistent timeline tree let designers fork, scrub, merge, and even message alternate branches.
- **Culture On Record** – Every major decision lives in `docs/echo` and the Neo4j devlog, giving future Echo pilots a map of our intent.

## Getting Started
```bash
git clone git@github.com:flyingrobots/EchoEngine.git
cd EchoEngine
npm install
```

### Scripts
| Command            | Description                                   |
| ------------------ | --------------------------------------------- |
| `npm run build`    | Build `@echo/core` once                        |
| `npm run dev`      | Watch `@echo/core` (TS incremental build)      |
| `npm run lint`     | Lint TypeScript sources                        |
| `npm run test`     | (Reserved) Vitest suite for `@echo/core`       |
| `npm run format`   | Prettier sweep over TypeScript sources         |

## Repository Layout
```
packages/
  echo-core/          # Runtime spine (ECS, scheduler, timelines)
apps/
  playground/         # Vite-powered sandbox (stub for now)
docs/
  echo/               # Specs, diagrams, memorials, excavation log
  legacy/             # Preserved Caverns relics
AGENTS.md             # Expectations for LLM & human collaborators
```

## Development Principles
1. **Tests First** – Write failing unit/integration/branch tests before new engine work.
2. **Branch Discipline** – Feature branches target `main`; keep `main` pristine.
3. **Document Ruthlessly** – Update specs/diagrams and log decisions in Neo4j (`scripts/neo4j-msg.js` living in `agent-collab`).
4. **Temporal Mindset** – Think Chronos (sequence), Kairos (possibility), Aion (significance) whenever touching runtime code.
5. **Journal Everything** – Every session, log a `[Echo]` entry to Neo4j noting who/what/why; future Echo pilots depend on it.

## Learning the Vision
- Read [`docs/echo/architecture-outline.md`](docs/echo/architecture-outline.md) for the full spec (storage, scheduler, ports, timelines).
- Explore [`docs/echo/diagrams.md`](docs/echo/diagrams.md) for Mermaid visuals of system constellations and the Chronos loop.
- Honor Caverns with [`docs/echo/memorial.md`](docs/echo/memorial.md)—we carry the torch forward.
- Peek at [`docs/echo/legacy-excavation.md`](docs/echo/legacy-excavation.md) to see which ideas survived the archaeological roast.

## Contributing
- Start each task by verifying a clean git state and branching (`echo/<feature>` recommended).
- Tests go in `packages/echo-core/test/` (fixtures in `test/fixtures/`). End-to-end scenarios will eventually live under `apps/playground`.
- Use expressive commits (`subject` / `body` / optional `trailer`)—tell future us the *why*, not just the *what*.
- Treat determinism as sacred: prefer Echo’s PRNG, avoid non-deterministic APIs without wrapping them.

## Roadmap Highlights
- **Phase 0** – Finalize ECS storage & scheduler designs, prototype benchmarks.
- **Phase 1** – Ship Echo Core MVP with tests and headless harness.
- **Phase 2 “Double-Jump”** – Deliver reference render/input adapters and the playground.
- **Phase 3+** – Physics, WebGPU, audio, inspector, and full temporal tooling.

Chrononauts welcome. Strap in, branch responsibly, and leave the timeline cleaner than you found it. 🌀
