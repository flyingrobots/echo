# RMG Viewer Roadmap

## P0 (must ship)
- [ ] Arrowheads offset so they stop at node radius (no penetration)
- [ ] VSync toggle (Immediate/AutoNoVsync vs Fifo) exposed in HUD
- [ ] Help/controls overlay (WASD/QE, L-drag look, R-drag spin, wheel FoV, debug toggles)
- [ ] HUD watermark using `docs/assets/ECHO.svg`
- [ ] Anti-aliased lines/wireframe (MSAA or FXAA) to reduce jaggies

## P1 (nice for initial release)
- [ ] Persist camera + HUD debug settings between runs (config file)
- [ ] Perf overlay with FPS + frame-time graph (egui_plot) and basic CPU/GPU timings
- [ ] Arrowhead scale/color tunable in HUD
- [ ] Camera near/far auto-fit from scene bounds (tighter depth range)

## P2 (next iterations)
- [ ] Real Echo/RMG snapshot loading (replace sample graph)
- [ ] Text/billboard labels for selected/hovered nodes
- [ ] SSAO or simple rim light for depth cueing
- [ ] Wireframe toggle for whole graph

## P3 (later polish)
- [ ] Arc edges (curved) option
- [ ] Multi-light / HDR tonemap option
- [ ] VR-friendly camera roll + pose export
