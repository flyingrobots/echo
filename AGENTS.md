# Echo Agent Briefing

Welcome to the **Echo** project. This file captures expectations for any LLM agent (and future-human collaborator) who touches the repo.

## Core Principles
- **Honor the Vision**: Echo is a deterministic, multiverse-aware ECS. Consult `docs/echo/architecture-outline.md` before touching runtime code.
- **Document Ruthlessly**: Every meaningful design choice should land in `docs/echo/` (spec, diagrams, memorials) or a Neo4j journal entry tagged `Echo`.
- **Determinism First**: Avoid introducing sources of nondeterminism without a mitigation plan.
- **Temporal Mindset**: Think in timelines—branching, merging, entropy budgets. Feature work should map to Chronos/Kairos/Aion axes where appropriate.

## Shared Memory (Neo4j)
We use the agent-collab Neo4j instance as a temporal journal.

Scripts live in `/Users/james/git/agent-collab/scripts/neo4j-msg.js`.

### Setup
```bash
# Register yourself once. Choose a display name that identifies the agent.
node /Users/james/git/agent-collab/scripts/neo4j-msg.js agent-init "Echo Codex"
```

### Writing a Journal Entry
```bash
node /Users/james/git/agent-collab/scripts/neo4j-msg.js msg-send \
  --from "Echo Codex" \
  --to "Echo Archive" \
  --text "[Echo] short summary of what you changed or decided." \
  --thread "echo-devlog" \
  --topic "Echo" \
  --project "Echo Engine" \
  --kind note
```

Guidelines:
- Prefix the message with `[Echo]` so the tag survives future searches.
- Summarise intent, work done, and next steps for future agents.
- Use the thread `echo-devlog` unless a more specific thread already exists.

### Reading Past Entries
```bash
node /Users/james/git/agent-collab/scripts/neo4j-msg.js messages \
  --thread "echo-devlog" \
  --limit 20
```

Use `messages-search --text "Echo"` for ad-hoc queries.

## Repository Layout
- `packages/echo-core`: Runtime core (ECS, scheduler, Codex’s Baby, timelines).
- `apps/playground`: Vite sandbox and inspector (future).
- `docs/echo`: Specs, diagrams, memorials.
- `docs/legacy`: Preserved artifacts from the Caverns era.

## Working Agreement
- Keep `main` pristine. Feature work belongs on branches named `echo/<feature>` or `timeline/<experiment>`.
- Tests and benchmarks are mandatory for runtime changes once the harness exists.
- Update the Neo4j log before you down tools, even if the work is incomplete.
- Respect determinism: preferably no random seeds without going through the Echo PRNG.

## Contact Threads
- Neo4j Thread `echo-devlog`: Daily journal, decisions, blockers.
- Neo4j Thread `echo-spec`: High-level architectural proposals.

Safe travels in the multiverse. Logged timelines are happy timelines. 🌀
