<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# World API Specification (Phase 0.5)

> **Background:** For a gentler introduction, see [WARP Primer](/guide/warp-primer).

Defines the public façade for interacting with Echo. External modules use this API while internals remain swappable.

---

## Goals

- Provide stable entry points for entity/component operations, event emission, branch management, replay, and inspection.
- Enforce determinism invariants at the boundary.
- Versioned to allow evolution without breaking user code.

---

## API Surface

```ts
interface EchoWorldAPI {
    version: string; // semantic version of API facade

    // ECS operations
    createEntity(archetype: ArchetypeDef): EntityId;
    destroyEntity(id: EntityId): void;
    addComponent<T>(id: EntityId, component: T): void;
    removeComponent(id: EntityId, type: ComponentTypeId): void;
    getComponent<T>(id: EntityId, type: ComponentTypeId): T | null;
    query<Q extends QuerySpec>(spec: Q): QueryResult<Q>;

    // Event system
    emit<T>(phase: SchedulerPhase, evt: EventEnvelope<T>): void;
    emitCross<T>(evt: EventEnvelope<T>): void;
    registerHandler(handler: EventHandler): () => void;

    // Timeline operations
    fork(fromNode?: NodeId): BranchId;
    merge(into: BranchId, from: BranchId): MergeResult;
    collapse(branch: BranchId): void;

    // Replay & verification
    replay(options: ReplayOptions): VerificationReport;

    // Inspection
    inspect(tick?: ChronosTick): InspectorFrame;
    inspectCodex(branch: BranchId): CBInspectorFrame;
    inspectBridge(): BridgeInspectorFrame;
}
```

### ReplayOptions

```ts
interface ReplayOptions {
    from: NodeId;
    until?: NodeId;
    verify?: boolean;
}
```

---

## Determinism Enforcement

- All mutations funnel through Codex’s Baby (`emit/emitCross`); direct ECS modifications prohibited.
- API ensures capability checks occur before operations (see [spec-capabilities-and-security.md](spec-capabilities-and-security.md)).
- `version` increments when breaking changes occur; components may opt into new versions explicitly.

---

## Examples

```ts
const api = createEchoWorld();
const player = api.createEntity(PlayerArchetype);
api.addComponent(player, Transform.default());
api.emit("update", {
    id: 0,
    kind: "input/keyboard",
    chronos: engine.currentTick + 1,
    kairos: engine.currentBranch,
    payload: { key: "Space", state: "down" },
});
```

---

## Change Management

This API follows [SemVer 2.0.0](https://semver.org/). Deprecated methods are retained as no-ops through the next major release. The `api.debug.*` namespace is `@unstable` with no compatibility guarantees.

- API changes require a version bump.
- Deprecated methods remain as no-ops until the next major release.
- Extensions (e.g., debug utilities) are provided under `api.debug.*` and marked `@unstable`.

---

This façade shields external consumers from internal architectural shifts while enforcing Echo’s determinism invariants.
