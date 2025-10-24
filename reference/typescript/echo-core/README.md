# @echo/core

Early scaffolding for the Echo engine. The public API is intentionally tiny while the
architecture takes shape:

- Deterministic tick loop placeholder via `EchoEngine`.
- Timeline fingerprints (`Chronos`, `Kairos`, `Aion`) as typed primitives.
- Command envelopes mirroring Codex's Baby event bus.

Nothing here is stable yet—expect rapid iteration as we land the ECS storage, scheduler,
and branch tree implementations.

```ts
import { EchoEngine } from "@echo/core";

const engine = new EchoEngine({ fixedTimeStepMs: 1000 / 30 });
engine.registerTickHandler(({ fingerprint, deltaTimeMs }) => {
  console.log("Tick", fingerprint, deltaTimeMs);
});

engine.tick();
```

## Dev Scripts

| Script       | Purpose                       |
| ------------ | ----------------------------- |
| `build`      | Emit `dist/` via TypeScript.  |
| `dev`        | Watch mode for rapid typings. |
| `lint`       | ESLint with TS rules.         |
| `test`       | Placeholder for Vitest suite. |
| `format`     | Prettier over sources.        |
