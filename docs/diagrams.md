<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Echo Diagram Vault

This folder sketches Echo’s moving parts using Mermaid. Each diagram matches the architecture spec and will eventually power an animated viewer (GSAP + SVG) once we export the Mermaid graphs.

> **Tip:** In VS Code or GitHub you can render these diagrams directly. For custom themes, we’ll feed the Mermaid JSON definitions into the web viewer later.

---

## 1. System Constellation

```mermaid
graph LR
  classDef core fill:#111827,stroke:#1f2937,color:#f9fafb,font-weight:600;
  classDef port fill:#0f172a,stroke:#1d4ed8,color:#bfdbfe,stroke-width:1.5px;
  classDef adapter fill:#1e293b,stroke:#94a3b8,color:#e2e8f0;
  classDef tool fill:#0f766e,stroke:#2dd4bf,color:#ecfeff;
  classDef service fill:#3f3a3a,stroke:#fcd34d,color:#fef3c7;

  subgraph Core["Echo Core"]
    ECS["@EntityComponentStore"]
    Scheduler["Scheduler\n(DAG + Branch Orchestrator)"]
    Codex["Event Bus\n(MaterializationBus)"]
    Timeline["Timeline Tree\n(Chronos/Kairos/Aion)"]
    Math["Deterministic Math\n(Vector, PRNG, Metrics)"]
    ECS --> Scheduler
    Scheduler --> Codex
    Scheduler --> Timeline
    Scheduler --> Math
  end
  class ECS,Scheduler,Codex,Timeline,Math core;

  subgraph Ports["Ports (Hexagonal boundary)"]
    RendererPort
    InputPort
    PhysicsPort
    AudioPort
    PersistencePort
    NetworkPort
  end
  class RendererPort,InputPort,PhysicsPort,AudioPort,PersistencePort,NetworkPort port;

  subgraph Adapters["Adapters"]
    RendererPort --> PixiAdapter["Pixi/WebGL Adapter"]
    RendererPort --> WebGPUAdapter["WebGPU Adapter"]
    RendererPort --> TUIGraphics["TUI Adapter"]

    InputPort --> BrowserInput["Browser Input"]
    InputPort --> NativeInput["SDL/Tauri Input"]
    InputPort --> AIInput["LLM Strategist"]

    PhysicsPort --> Box2DAdapter
    PhysicsPort --> RapierAdapter
    PhysicsPort --> DeterministicSolver

    AudioPort --> WebAudioAdapter
    AudioPort --> NativeAudioAdapter

    PersistencePort --> LocalStorageAdapter
    PersistencePort --> CloudAdapter

    NetworkPort --> WebRTCAdapter
    NetworkPort --> DedicatedServerAdapter
  end
  class PixiAdapter,WebGPUAdapter,TUIGraphics,BrowserInput,NativeInput,AIInput,Box2DAdapter,RapierAdapter,DeterministicSolver,WebAudioAdapter,NativeAudioAdapter,LocalStorageAdapter,CloudAdapter,WebRTCAdapter,DedicatedServerAdapter adapter;

  subgraph Tooling["Tooling & Observability"]
    Inspector["Echo Inspector"]
    TimelineViewer["Timeline Vault"]
    Benchmarks["Benchmark Suite"]
    Editor["Echo Studio"]
  end
  class Inspector,TimelineViewer,Benchmarks,Editor tool;

  subgraph Services["Cross-Cutting Services"]
    Config
    DI["Dependency Injector"]
    Entropy["Entropy Monitor"]
    Diagnostics["Telemetry/Logging"]
  end
  class Config,DI,Entropy,Diagnostics service;

  Ports -. APIS .-> Core
  Core -- Events/Commands --> Ports
  Tooling --- Core
  Services --- Core
  Services --- Tooling
```

---

## 2. Chronos Loop (Single Frame, Single Branch)

```mermaid
flowchart TD
  classDef stage fill:#1e293b,stroke:#e0f2fe,color:#bae6fd,font-weight:600;
  classDef phase fill:#0f172a,stroke:#f97316,color:#fb923c,font-weight:500;
  classDef op fill:#312e81,stroke:#a78bfa,color:#ede9fe;
  classDef sub fill:#111827,stroke:#6366f1,color:#c7d2fe,font-style:italic;

  Start((Start Tick)):::stage --> Clock["Clock\nAccumulate dt"]:::phase
  Clock -->|dt| SchedulerPre["Phase 1: Pre-Update"]:::stage

  SchedulerPre --> InputAssim["Assimilate Input\n(InputPort flush)"]:::op
  InputAssim --> CodexPre["Event Bus\nPre-Flush"]:::op
  CodexPre --> TimelineIntake["Timeline Tree\nRegister Branch Jobs"]:::op

  TimelineIntake --> UpdatePhase["Phase 2: Update Systems"]:::stage
  UpdatePhase --> DAG["Resolve DAG\n(Dependencies)"]:::op
  DAG --> ParallelBatch["Plan Parallel Batches"]:::op
  ParallelBatch --> SystemsLoop{"For each batch"}:::phase
  SystemsLoop -->|system| SystemExec["Run System\n(Query + Mutate ECS)\nUpdate Codex"]:::op
  SystemExec --> SystemsLoop

  SystemsLoop --> PostUpdate["Phase 3: Post-Update"]:::stage
  PostUpdate --> Hooks["Late Hooks\n(Animation, Cleanups)"]:::op
  Hooks --> PhysicsSync["Physics Sync"]:::op
  PhysicsSync --> MathResolve["Math Snap (fround/fixed-point)"]:::op

  MathResolve --> RenderPrep["Phase 4: Render Prep"]:::stage
  RenderPrep --> FramePacket["Assemble FramePacket\n(Query renderer views)"]:::op
  FramePacket --> DiagnosticsStage["Dev Diagnostics"]:::op

  DiagnosticsStage --> Present["Phase 5: Present"]:::stage
  Present --> RendererCall["RendererPort.submit(frame)"]:::op

  RendererCall --> TimelineFlush["Phase 6: Timeline Flush"]:::stage
  TimelineFlush --> DiffPersist["Persist Diffs\n(COW chunks, diff cache)"]:::op
  DiffPersist --> EntropyUpdate["Update Entropy/Aion Metrics"]:::op
  EntropyUpdate --> BranchBook["Update Branch Index"]:::op

  BranchBook --> End((End Tick)):::stage
```

---

## 3. Multiverse Mesh (Branch Tree)

```mermaid
graph TD
  classDef base fill:#111111,stroke:#6b7280,color:#f5f5f5,font-weight:600;
  classDef node fill:#0f172a,stroke:#38bdf8,color:#e0f2fe;
  classDef merge fill:#422006,stroke:#f97316,color:#fed7aa;
  classDef ghost fill:#312e81,stroke:#c084fc,color:#ede9fe;

  subgraph TimelineTree["Persistent Timeline Tree"]
    Root["C0\n(Chronos=0,\nKairos=Prime,\nAion=Baseline)"]:::base
    Root --> N1["C15 Kα A0.8\n\"Puzzle Attempt\""]:::node
    Root --> N2["C15 Kβ A0.2\n\"Alt Strategy\""]:::node
    N1 --> N1a["C24 Kα1 A0.95\n\"Boss Victory\""]:::node
    N1 --> N1b["C24 Kα2 A0.6\n\"Loot Run\""]:::node
    N2 --> N2a["C20 Kβ1 A0.3\n\"Reverse Time Room\""]:::node
    N2a --> MergeCandidate{{Merge?\nΔconflict=low}}:::merge
    MergeCandidate --> N3["C32 Kγ A0.9\n\"Braided Outcome\""]:::node
    N1b -. Ghost Echo .-> N3
    N2a -. ParadoxFlag .-> N3
  end

  class MergeCandidate merge;
  class N1,N2,N1a,N1b,N2a,N3 node;
  class Root base;
```

---

## 4. Message Bridge Across Branches

```mermaid
sequenceDiagram
  autonumber
  participant BranchAlpha as Branch α (C24)
  participant CodexAlpha as Codex α
  participant Bridge as Temporal Bridge
  participant CodexBeta as Codex β
  participant BranchBeta as Branch β (C18)

  BranchAlpha->>CodexAlpha: enqueue PastMessage{target=C12, payload=hint}
  CodexAlpha->>Bridge: dispatch envelope (Chronos=24, Kairos=α, Aion=0.8)
  Bridge->>Bridge: validate paradox risk / entropy cost
  Bridge->>CodexBeta: spawn retro branch at C12
  CodexBeta->>BranchBeta: deliver PastMessage at Chronos=12
  BranchBeta->>Bridge: acknowledge timeline fork (Kairos=β′)
  Note over BranchAlpha,BranchBeta: Player can merge β′ back into α if conflicts resolved
```

---

## Animation Ideas

- **GSAP Morphs**: Export Mermaid SVG and tween branch nodes as timelines split/merge.
- **Entropy Pulse**: Animate stroke width/color based on the Entropy meter.
- **Interactive Sequencer**: Play back the sequence diagram with tooltips showing Codex queue sizes.

Once the architecture crystallizes, we’ll wire these into a future documentation viewer/playground that live-updates from this Markdown.
