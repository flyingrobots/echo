<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# RMG Viewer Roadmap (reprioritized)

The viewer must stay a rendering adapter. Session logic, persistence, and notifications live in shared core/service crates; the viewer only draws and emits UI events.

## Done / Baseline
- [x] Arrowheads offset stop at node radius (no penetration)
- [x] VSync toggle in HUD (Immediate/AutoNoVsync vs Fifo)
- [x] Help/controls overlay (WASD/QE, L-drag look, R-drag spin, wheel FoV, debug toggles)
- [x] HUD watermark using `docs/assets/ECHO.svg`
- [x] Anti-aliased lines/wireframe (MSAA)
- [x] Persist camera + HUD debug settings between runs (temporary in-viewer impl)

## P0 — Architectural realignment (in-process)
- [x] Add `echo-app-core` crate with `ToastService`, `ConfigService`, ports (`NotificationSinkPort`, `ConfigStorePort`), and `ViewerPrefs` data.
- [x] Add filesystem adapter (`echo-config-fs`) implementing `ConfigStorePort` (directories + serde_json).
- [x] Refactor `rmg-viewer` to a pure rendering adapter: remove serde/directories usage; accept injected `ViewerPrefs`; emit prefs on exit.
- [ ] HUD toast renderer that consumes toasts supplied by core; no toast creation inside viewer.
 - [x] HUD toast renderer that consumes toasts supplied by core; no toast creation inside viewer.

## P1 — Distributed session/service slice
- [x] Define `echo-session-proto` wire schema (Hello, RegisterRmg, RmgDiff/Snapshot, Command/Ack, Notification).
- [x] Ship `echo-session-service` (headless hub) hosting session/core services over Unix socket/pipe. *(skeleton placeholder; transport TBD)*
- [x] Ship `echo-session-client` crate for tools (viewer, game, inspector) with local loopback fallback. *(stub APIs; transport TBD)*
- [ ] Convert `rmg-viewer` to consume RMG streams + notifications via client; keep sample graph as offline fallback.

## P2 — Viewer UX & diagnostics
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
