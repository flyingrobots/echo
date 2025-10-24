# Networking Specification (Phase 0.75)

Defines Echo’s deterministic networking model based on event replication, rollback, and branch merges.

---

## Core Principle
Networking transports `EventEnvelope`s; no raw state replication. Every node runs the same simulation, receiving identical events in deterministic order.

---

## Architecture Layers

| Layer | Responsibility | Language |
| ----- | -------------- | -------- |
| Networking Core | Event replication, lockstep/rollback, authority decisions | Rust |
| Codex’s Baby Bridge | Converts network packets into cross-branch events | Rust / Lua |
| Lua Gameplay | Declares networked components/events via API | Lua |

---

## Modes
1. **Lockstep** – Collect inputs for tick `n` from all peers, then advance. Perfect determinism, higher latency.
2. **Rollback (Predictive)** – Predict local inputs for a window. When authoritative events arrive, rollback to LCA tick and replay deterministically using branch tree capabilities.
3. **Authoritative Hybrid** – Host/server acts as merge authority, selecting canonical branch and rejecting paradoxes.

---

## Networking Port API

```ts
interface NetworkingPort {
  mode: "p2p" | "client-server";
  send(evt: EventEnvelope): void;
  receive(): EventEnvelope[];
  syncClock(): ChronosTick;
}
```

- Transports (WebRTC, UDP, etc.) feed canonical events.
- Packets include serialized `EventEnvelope` using canonical encoder.
- Capability tokens guard network usage (`network:emit`, `network:authority`).

---

## Lua API Surface

```lua
function on_start()
  echo.network.emit("player_input", {
    axis = self.move,
    tick = echo.chronos()
  })
end

function on_player_input(evt)
  self:applyInput(evt.payload)
end
```

- Lua never opens sockets; it emits/handles events.
- Engine assigns Chronos/Kairos IDs and handles delivery/rollback.

---

## Determinism Constraints
- All network data serialized via canonical encoder; hashed for verification.
- Clock sync uses tick counts, not wall time.
- Packet loss handled via resend; dedup through `envelopeHash`.
- Randomness seeded from branch IDs; peers share identical seeds.

---

## Tooling Hooks
- Network debugger visualizes branch timelines, latency, rollback steps.
- CLI: `echo net replay --log file.jsonl` replays recorded network event streams.

---

This spec maintains Echo’s deterministic guarantees across multiplayer scenarios by treating networking as branch synchronization.
