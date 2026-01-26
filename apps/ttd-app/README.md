<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo TTD App

Browser-based Time-Travel Debugger for Echo applications.

## Status

**Scaffolded** — UI framework in place with mock data. Waiting for:

- `ttd-protocol-ts` from Wesley Phase 1b (generated TypeScript types)
- `ttd-browser` WASM build (Task 5.4 complete, needs `wasm-pack build`)

## Architecture

```text
┌──────────────────────────────────────────────────────────────────┐
│  ttd-app (React + Vite)                                          │
├──────────────────────────────────────────────────────────────────┤
│  Views                                                           │
│    └── Layout.tsx (grid: header, sidebars, center, footer)       │
├──────────────────────────────────────────────────────────────────┤
│  Components                                                       │
│    ├── TimeControls.tsx   (play/pause/step/seek, speed, fork)    │
│    ├── Timeline.tsx       (scrubber, markers for events)         │
│    ├── WorldlineTree.tsx  (fork hierarchy, compliance badges)    │
│    ├── StateInspector.tsx (atom table with values)               │
│    └── ProvenanceDrawer.tsx (atom history, causal chain)         │
├──────────────────────────────────────────────────────────────────┤
│  Hooks & Store                                                    │
│    ├── useTtdEngine.ts    (WASM engine wrapper)                  │
│    └── ttdStore.ts        (Zustand state management)             │
├──────────────────────────────────────────────────────────────────┤
│  Types                                                            │
│    └── ttd.ts             (placeholder types for Wesley)         │
├──────────────────────────────────────────────────────────────────┤
│  External Dependencies                                            │
│    ├── ttd-browser        (WASM - cursor/session/provenance)     │
│    └── @echo/renderer-three (Three.js 4D visualization)          │
└──────────────────────────────────────────────────────────────────┘
```

## Panels

| Panel             | Purpose                                    |
| ----------------- | ------------------------------------------ |
| Time Controls     | Play/Pause/Step/Seek, speed, fork button   |
| Timeline          | Visual scrubber with event markers         |
| Worldline Tree    | Fork hierarchy with compliance badges      |
| State Inspector   | Atom table showing current values          |
| Provenance Drawer | Slide-out showing atom write history       |
| 4D View           | Three.js spacetime visualization (planned) |

## Development

```bash
# Install dependencies
pnpm install

# Start dev server
pnpm dev

# Type check
pnpm typecheck

# Build for production
pnpm build
```

## Connecting Real Data

1. **Build ttd-browser WASM:**

    ```bash
    cd ../../crates/ttd-browser
    wasm-pack build --target web
    ```

2. **Update `useTtdEngine.ts`:**

    ```typescript
    // Replace mock engine with real WASM import
    const { default: init, TtdEngine } = await import("ttd-browser");
    await init();
    const engine = new TtdEngine();
    ```

3. **Replace placeholder types** with Wesley-generated `ttd-protocol-ts` once available.

## File Structure

```text
apps/ttd-app/
├── index.html
├── package.json
├── tsconfig.json
├── tsconfig.node.json
├── vite.config.ts
├── public/
└── src/
    ├── main.tsx
    ├── App.tsx
    ├── index.css
    ├── components/
    │   ├── TimeControls.tsx / .css
    │   ├── Timeline.tsx / .css
    │   ├── WorldlineTree.tsx / .css
    │   ├── StateInspector.tsx / .css
    │   └── ProvenanceDrawer.tsx / .css
    ├── hooks/
    │   └── useTtdEngine.ts
    ├── store/
    │   └── ttdStore.ts
    ├── types/
    │   └── ttd.ts
    └── views/
        ├── Layout.tsx
        └── Layout.css
```

## Related

- `crates/ttd-browser/` — WASM engine (Task 5.4)
- `packages/echo-renderer-three/` — Three.js adapter (Task 6.2)
- `docs/plans/ttd-app.md` — Full specification

## License

Apache-2.0
