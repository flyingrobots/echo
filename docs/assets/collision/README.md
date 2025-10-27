# Collision/CCD DPO Diagrams

- Base CSS: `diagrams.css`
- Static SVGs:
  - `dpo_build_temporal_proxy.svg`
  - `dpo_broad_phase_pairing.svg`
  - `dpo_narrow_phase_discrete.svg`
  - `dpo_narrow_phase_ccd.svg`
  - `dpo_contact_events.svg`
  - `dpo_gc_ephemeral.svg`
  - `scheduler_phase_mapping.svg`
  - `legend.svg`

Each SVG uses semantic classes:
- Nodes: `node`, variants: `interfaceK`, `added`, `removed`
- Edges: `edge`, variants: `added`, `removed`
- Other: `title`, `label`, `caption`, `scope`
- Optional animations: `pulse-add`, `pulse-remove`

## Animations
- Extend `diagrams.css` with your own `@keyframes` and apply classes to elements.
- For step-by-step tutorials, use the `_step1.svg`, `_step2.svg`, `_step3.svg` variants or overlay multiple SVGs in the docs site.

## Mermaid Sources
- Source `.mmd` files live alongside these SVGs. You can generate SVGs locally:

```
# Requires Node.js
npm i -g @mermaid-js/mermaid-cli
mmdc -i build_temporal_proxy.mmd -o build_temporal_proxy.svg
```

CI also compiles `.mmd` files to SVG artifacts (see `.github/workflows/ci.yml` job `diagrams`).
