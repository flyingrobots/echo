# Entropy & Paradox Specification (Phase 0.75)

Defines the entropy model, paradox lifecycle, and observer APIs that turn temporal instability into first-class simulation data.

---

## Goals
- Quantify simulation instability with deterministic formulae.
- Detect, quarantine, and resolve paradoxes consistently across branches.
- Provide hooks for gameplay systems, AI agents, and inspector tooling.

---

## Core Concepts
- **Entropy (Aion weight):** scalar measure of timeline instability.
- **Paradox:** causal violation where a write modifies data previously read by earlier diffs in the same Chronos window.
- **Stabilizer:** event or system that intentionally reduces entropy (e.g., paradox resolution).

---

## Entropy Formulae

```text
entropyΔ = wF*forks + wM*merges + wP*paradoxes + wB*bridgeMsgs − wS*stabilizers
entropyTotal(t) = Σ entropyΔ over Chronos ≤ t
```

Default weights (configurable): `wF=1`, `wM=1`, `wP=2`, `wB=1`, `wS=2`.
- Forks: branch creations.
- Merges: successful merges.
- Paradoxes: quarantined events/diffs.
- BridgeMsgs: cross-branch events delivered.
- Stabilizers: successful paradox resolutions or entropy-reducing quests.

Entropy clamped `[0, +∞)`; inspector visualizes normalized values.

---

## Paradox Lifecycle

1. **Detection** – compare diff read/write sets: `writesB ∩ readsA ≠ ∅` with `Chronos(A) < Chronos(B)`.
2. **Quarantine** – emit `ParadoxNode`:
   ```ts
   interface ParadoxNode {
     id: string;
     offendingEvent: EventEnvelope;
     readKeys: ReadKey[];
     writeKeys: WriteKey[];
     branch: KairosBranchId;
     entropyDelta: number;
     resolved?: boolean;
     resolution?: "rollback" | "merge" | "ignore";
   }
   ```
3. **Resolution** – manual or automated strategy reduces entropy (subtract `wS`).
4. **Logging** – paradox nodes recorded in branch tree and inspector; decisions hashed for replay.

---

## APIs

```ts
interface EntropyObserver {
  onEntropyChange(node: TimelineNode, delta: number, total: number): void;
}

interface ParadoxService {
  detect(diffA: DiffRecord, diffB: DiffRecord): ParadoxNode[];
  quarantine(node: ParadoxNode): void;
  resolve(id: string, strategy: "rollback" | "merge" | "ignore"): void;
  list(branch?: KairosBranchId): ParadoxNode[];
}
```

Observers register with branch tree; notifications occur each `timeline_flush`.

---

## CLI & Inspector
- `echo entropy --branch <id>` – prints entropy history and stats.
- `echo paradoxes` – lists unresolved paradox nodes.
- Inspector frames provide entropy timelines, branch heatmaps, and paradox markers.

```ts
interface EntropyFrame {
  tick: ChronosTick;
  branch: KairosBranchId;
  delta: number;
  total: number;
}

interface ParadoxFrame {
  tick: ChronosTick;
  paradoxes: ParadoxNode[];
}
```

---

## Determinism
- Entropy deltas recorded as deterministic values in diffs.
- Paradox resolution decisions stored in merge metadata and hashed.
- Replays reproduce identical entropy curves and paradox sequences.

---

This spec centralizes entropy management, ensuring causal violations are tracked, exposed, and resolved deterministically.
