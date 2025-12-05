<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# RMG Viewer Roadmap (reprioritized)

The viewer must stay a rendering adapter. Session logic, persistence, and notifications live in shared core/service crates; the viewer only draws and emits UI events.

## P0 — Architectural realignment (complete)

## P1 — Architecture (hex) + session slice
- [ ] Scene conversion: improve `scene_from_wire` to use real payloads (positions/colors) instead of placeholder radial layout.
- [ ] Hex refactor inside viewer:
  - [x] Introduce `Viewport` wrapper (window + gpu + egui + render port); migrate App to `Vec<Viewport>` for future multi-window/multiview support.
  - [x] Extract domain core/state machine + effects (pure transitions) into its own module; leave `main.rs` as composition only. (`App` moved to `app.rs`, `ViewerState` to `viewer_state.rs`, `Viewport` to `viewport.rs`)
  - [x] Add `UiEvent`/`UiEffect` reducer + effect runner; make UI strictly unidirectional.
  - [ ] Route UI actions through ports (`SessionPort`, `ConfigPort`, `RenderPort`) instead of direct calls. *(RenderPort still bypassed for redraw; menu overlays still call viewer state directly in places)*
  - [ ] Define RenderPort adapter usage (trait exists in core; viewer still calls winit directly) and remove raw redraw calls.
  - [ ] Move UI rendering into ui adapter; move session IO to session adapter; move wgpu passes to render adapter; keep `app.rs` minimal (per-frame loop still lives in `app.rs`, could be split into `app_frame.rs`/`app_events.rs`).

## P2 — Viewer UX & diagnostics
- [ ] New screen flows per spec:
  - Title: wordmark/version, menu (Connect / Settings / Exit).
  - Connect form: host/port/rmg-id; transitions to Connecting.
  - Settings overlay reused in Title/Viewer; Save/Back behaviors.
  - Connecting screen with boot-style log; transitions to Viewer on success.
  - Error screen fallback with reconnect path.
- [ ] Viewer menu overlays: Menu button opens Settings / Publish Local RMG / Subscribe to RMG / Back; overlays leave HUD/watermark visible.
- [ ] Subscribe overlay (RMG directory): list known RmgIds from host, buttons to subscribe/unsubscribe, Back returns to Viewer HUD.
- [ ] Publish mode toggle: mark this client as producer for chosen RmgId and stream local RMG.
- [ ] RMG ring layout in scene: place local+subscribed RMGs on a circle, arrow keys rotate ring to focus a selected RMG; update camera focus accordingly.
- [ ] Perf overlay with FPS + frame-time graph (egui_plot) and basic CPU/GPU timings.
- [ ] Arrowhead scale/color tunables in HUD.
- [ ] Camera near/far auto-fit from scene bounds (tighter depth range).
- [ ] Wireframe toggle for whole graph.
- [ ] Text/billboard labels for selected/hovered nodes.

## P3 — Visual depth cues
- [ ] SSAO or simple rim light for depth cueing.
- [ ] Arc edges (curved) option.

## P4 — Later polish
- [ ] Multi-light / HDR tonemap option.
- [ ] VR-friendly camera roll + pose export.

## Done
- [x] Arrowheads offset stop at node radius (no penetration)
- [x] VSync toggle in HUD (Immediate/AutoNoVsync vs Fifo)
- [x] Help/controls overlay (WASD/QE, L-drag look, R-drag spin, wheel FoV, debug toggles)
- [x] HUD watermark using `docs/assets/ECHO.svg`
- [x] Anti-aliased lines/wireframe (MSAA)
- [x] Persist camera + HUD debug settings between runs (temporary in-viewer impl)

- [x] Add `echo-app-core` crate with `ToastService`, `ConfigService`, ports (`NotificationSinkPort`, `ConfigStorePort`), and `ViewerPrefs` data.
- [x] Add filesystem adapter (`echo-config-fs`) implementing `ConfigStorePort` (directories + serde_json).
- [x] Refactor `rmg-viewer` to a pure rendering adapter: remove serde/directories usage; accept injected `ViewerPrefs`; emit prefs on exit.
- [x] HUD toast renderer that consumes toasts supplied by core; no toast creation inside viewer.
- [x] Define `echo-session-proto` wire schema (Hello, RegisterRmg, RmgDiff/Snapshot, Command/Ack, Notification).
- [x] Ship `echo-session-service` (headless hub) hosting session/core services over Unix socket/pipe. *(skeleton placeholder; transport TBD)*
- [x] Ship `echo-session-client` crate for tools (viewer, game, inspector) with local loopback fallback. *(stub APIs; transport TBD)*
- [x] Introduce shared canonical `echo-graph` crate and move `RmgFrame`/`RmgOp`/`Snapshot`/`Diff` there; proto and viewer use it.
- [x] Convert `rmg-viewer` to consume RMG streams + notifications via the session client (no direct socket/CBOR in viewer).
- [x] Extract session IO from viewer into a thin adapter (injected ports): viewer takes notifications/RMG frames from outside; no socket/CBOR in viewer binary.
- [x] Engine emitter: send canonical `echo-graph::RmgFrame` (Snapshot first, then gapless Diff with `RmgOp`) over Unix socket; enforce no-gaps.
- [x] Viewer: decode real `RmgFrame` snapshots/diffs, apply structural ops to wire graph, rebuild scene; drop connection on gap/hash mismatch. *(disconnect + error overlay now wired)*
  - [x] Extract domain UI/core state machine container (`UiState`) to separate module.
  - [x] Encapsulate session IO channels in a `SessionClient` adapter (prep for SessionPort).
  - [x] Define `SessionPort` trait and implement for session adapter; draining moved out of `about_to_wait` loops.
