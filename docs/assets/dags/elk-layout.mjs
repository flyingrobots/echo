#!/usr/bin/env node
/**
 * ELK Layout Generator for Scene Port Adapter DAG
 * Uses elkjs for hierarchical graph layout
 */

import ELK from 'elkjs';

const elk = new ELK();

// Color palette by phase
const COLORS = {
  phase1: '#E8F5E9',  // Rust Port - light green
  phase2: '#C8E6C9',  // Rust Codec - green
  phase3: '#E3F2FD',  // TS Bootstrap - light blue
  phase4: '#BBDEFB',  // TS State - blue
  phase5: '#FFF3E0',  // TS Utils - light orange
  phase6: '#FFF3E0',  // TS Utils - light orange
  phase7: '#F8BBD9',  // TS Render - light pink
  phase8: '#F8BBD9',  // TS Render - light pink
  phase9: '#F8BBD9',  // TS Render - light pink
  phase10: '#F3E5F5', // Adapter - light purple
  phase11: '#FFF9C4', // Tests - light yellow
  phase12: '#FFF9C4', // Docs - light yellow
  milestone: '#E0E0E0',
};

// Define the graph structure
const graph = {
  id: 'root',
  layoutOptions: {
    'elk.algorithm': 'layered',
    'elk.direction': 'DOWN',
    'elk.layered.spacing.nodeNodeBetweenLayers': '60',
    'elk.layered.spacing.edgeNodeBetweenLayers': '30',
    'elk.spacing.nodeNode': '40',
    'elk.layered.nodePlacement.strategy': 'NETWORK_SIMPLEX',
    'elk.layered.crossingMinimization.strategy': 'LAYER_SWEEP',
    'elk.hierarchyHandling': 'INCLUDE_CHILDREN',
  },
  children: [
    // Phase 1: Rust Port Crate
    { id: 'SPA.1.1', width: 140, height: 50, labels: [{ text: 'SPA.1.1\nCreate Cargo.toml\n& lib.rs shell' }], phase: 'phase1' },
    { id: 'SPA.1.2', width: 140, height: 50, labels: [{ text: 'SPA.1.2\ntypes.rs\n(NodeDef, EdgeDef)' }], phase: 'phase1' },
    { id: 'SPA.1.3', width: 140, height: 50, labels: [{ text: 'SPA.1.3\ncamera.rs\n(CameraState)' }], phase: 'phase1' },
    { id: 'SPA.1.4', width: 140, height: 50, labels: [{ text: 'SPA.1.4\nhighlight.rs\n(HighlightState)' }], phase: 'phase1' },
    { id: 'SPA.1.5', width: 140, height: 50, labels: [{ text: 'SPA.1.5\ncanon.rs\n(float canon)' }], phase: 'phase1' },
    { id: 'SPA.1.6', width: 140, height: 50, labels: [{ text: 'SPA.1.6\nport.rs\n(ScenePort trait)' }], phase: 'phase1' },

    // Phase 2: Rust Codec
    { id: 'SPA.2.1', width: 140, height: 50, labels: [{ text: 'SPA.2.1\ncodec Cargo.toml\n& lib.rs' }], phase: 'phase2' },
    { id: 'SPA.2.2', width: 140, height: 50, labels: [{ text: 'SPA.2.2\ncbor.rs\n(CBOR codec)' }], phase: 'phase2' },
    { id: 'SPA.2.3', width: 140, height: 50, labels: [{ text: 'SPA.2.3\nMockAdapter\n(headless test)' }], phase: 'phase2' },
    { id: 'SPA.2.4', width: 140, height: 50, labels: [{ text: 'SPA.2.4\nroundtrip tests' }], phase: 'phase2' },
    { id: 'SPA.2.5', width: 140, height: 50, labels: [{ text: 'SPA.2.5\nMockAdapter tests' }], phase: 'phase2' },

    // Phase 3: TS Bootstrap
    { id: 'SPA.3.1', width: 140, height: 50, labels: [{ text: 'SPA.3.1\npackage.json\ntsconfig, vitest' }], phase: 'phase3' },
    { id: 'SPA.3.2', width: 140, height: 50, labels: [{ text: 'SPA.3.2\nSceneDelta.ts\n(+ hashToHex)' }], phase: 'phase3' },
    { id: 'SPA.3.3', width: 140, height: 50, labels: [{ text: 'SPA.3.3\nCameraState.ts' }], phase: 'phase3' },
    { id: 'SPA.3.4', width: 140, height: 50, labels: [{ text: 'SPA.3.4\nHighlightState.ts' }], phase: 'phase3' },
    { id: 'SPA.3.5', width: 140, height: 50, labels: [{ text: 'SPA.3.5\ntypes/index.ts\n(RenderContext)' }], phase: 'phase3' },

    // Phase 4: TS State
    { id: 'SPA.4.1', width: 140, height: 50, labels: [{ text: 'SPA.4.1\nSceneState.ts\n(state machine)' }], phase: 'phase4' },
    { id: 'SPA.4.2', width: 140, height: 50, labels: [{ text: 'SPA.4.2\nSceneState tests' }], phase: 'phase4' },

    // Phase 5-6: TS Utils
    { id: 'SPA.5.1', width: 140, height: 50, labels: [{ text: 'SPA.5.1\nShaderManager.ts\n(no singletons)' }], phase: 'phase5' },
    { id: 'SPA.6.1', width: 140, height: 50, labels: [{ text: 'SPA.6.1\nAssetManager.ts\n(URL-keyed)' }], phase: 'phase6' },

    // Phase 7: RenderCore
    { id: 'SPA.7.1', width: 140, height: 50, labels: [{ text: 'SPA.7.1\nThreeRenderCore.ts\n(WebGL wrapper)' }], phase: 'phase7' },
    { id: 'SPA.7.2', width: 140, height: 50, labels: [{ text: 'SPA.7.2\nCameraController.ts' }], phase: 'phase7' },

    // Phase 8: Renderers
    { id: 'SPA.8.1', width: 140, height: 50, labels: [{ text: 'SPA.8.1\nNodeRenderer.ts\n(spheres/boxes)' }], phase: 'phase8' },
    { id: 'SPA.8.2', width: 140, height: 50, labels: [{ text: 'SPA.8.2\nEdgeRenderer.ts\n(lines)' }], phase: 'phase8' },
    { id: 'SPA.8.3', width: 140, height: 50, labels: [{ text: 'SPA.8.3\nLabelRenderer.ts\n(sprites)' }], phase: 'phase8' },

    // Phase 9: Highlight
    { id: 'SPA.9.1', width: 140, height: 50, labels: [{ text: 'SPA.9.1\nHighlightRenderer.ts\n(color tint)' }], phase: 'phase9' },

    // Phase 10: Adapter
    { id: 'SPA.10.1', width: 160, height: 50, labels: [{ text: 'SPA.10.1\nThreeSceneAdapter.ts\n(ScenePort impl)' }], phase: 'phase10' },
    { id: 'SPA.10.2', width: 140, height: 50, labels: [{ text: 'SPA.10.2\nindex.ts exports' }], phase: 'phase10' },

    // Phase 11: Tests
    { id: 'SPA.11.1', width: 140, height: 50, labels: [{ text: 'SPA.11.1\nadapter.test.ts\n(integration)' }], phase: 'phase11' },
    { id: 'SPA.11.2', width: 140, height: 50, labels: [{ text: 'SPA.11.2\ndeterminism.test.ts\n(cross-check)' }], phase: 'phase11' },

    // Phase 12: Docs
    { id: 'SPA.12.1', width: 140, height: 50, labels: [{ text: 'SPA.12.1\nREADME.md' }], phase: 'phase12' },
    { id: 'SPA.12.2', width: 140, height: 50, labels: [{ text: 'SPA.12.2\nFinalize & verify' }], phase: 'phase12' },

    // Milestones
    { id: 'M1', width: 100, height: 60, labels: [{ text: 'M1\nRust crates\ncomplete' }], phase: 'milestone' },
    { id: 'M2', width: 100, height: 60, labels: [{ text: 'M2\nTS types\ncomplete' }], phase: 'milestone' },
    { id: 'M3', width: 100, height: 60, labels: [{ text: 'M3\nRenderers\ncomplete' }], phase: 'milestone' },
    { id: 'M4', width: 100, height: 60, labels: [{ text: 'M4\nShippable\npackage' }], phase: 'milestone' },
  ],
  edges: [
    // Phase 1 internal
    { id: 'e1', sources: ['SPA.1.1'], targets: ['SPA.1.2'] },
    { id: 'e2', sources: ['SPA.1.1'], targets: ['SPA.1.3'] },
    { id: 'e3', sources: ['SPA.1.1'], targets: ['SPA.1.5'] },
    { id: 'e4', sources: ['SPA.1.2'], targets: ['SPA.1.4'] },
    { id: 'e5', sources: ['SPA.1.2'], targets: ['SPA.1.6'] },
    { id: 'e6', sources: ['SPA.1.3'], targets: ['SPA.1.6'] },
    { id: 'e7', sources: ['SPA.1.4'], targets: ['SPA.1.6'] },

    // Phase 1 -> Phase 2
    { id: 'e8', sources: ['SPA.1.6'], targets: ['SPA.2.1'] },

    // Phase 2 internal
    { id: 'e9', sources: ['SPA.2.1'], targets: ['SPA.2.2'] },
    { id: 'e10', sources: ['SPA.2.2'], targets: ['SPA.2.3'] },
    { id: 'e11', sources: ['SPA.2.2'], targets: ['SPA.2.4'] },
    { id: 'e12', sources: ['SPA.2.3'], targets: ['SPA.2.5'] },

    // Phase 3 internal
    { id: 'e13', sources: ['SPA.3.1'], targets: ['SPA.3.2'] },
    { id: 'e14', sources: ['SPA.3.1'], targets: ['SPA.3.3'] },
    { id: 'e15', sources: ['SPA.3.2'], targets: ['SPA.3.4'] },
    { id: 'e16', sources: ['SPA.3.2'], targets: ['SPA.3.5'] },
    { id: 'e17', sources: ['SPA.3.3'], targets: ['SPA.3.5'] },
    { id: 'e18', sources: ['SPA.3.4'], targets: ['SPA.3.5'] },

    // Cross-phase mirrors (dashed)
    { id: 'e19', sources: ['SPA.1.2'], targets: ['SPA.3.2'], dashed: true },
    { id: 'e20', sources: ['SPA.1.3'], targets: ['SPA.3.3'], dashed: true },
    { id: 'e21', sources: ['SPA.1.4'], targets: ['SPA.3.4'], dashed: true },

    // Phase 4
    { id: 'e22', sources: ['SPA.3.2'], targets: ['SPA.4.1'] },
    { id: 'e23', sources: ['SPA.4.1'], targets: ['SPA.4.2'] },

    // Phase 5-6
    { id: 'e24', sources: ['SPA.3.1'], targets: ['SPA.5.1'] },
    { id: 'e25', sources: ['SPA.3.1'], targets: ['SPA.6.1'] },

    // Phase 7
    { id: 'e26', sources: ['SPA.3.1'], targets: ['SPA.7.1'] },
    { id: 'e27', sources: ['SPA.3.3'], targets: ['SPA.7.2'] },

    // Phase 8
    { id: 'e28', sources: ['SPA.3.2'], targets: ['SPA.8.1'] },
    { id: 'e29', sources: ['SPA.4.1'], targets: ['SPA.8.1'] },
    { id: 'e30', sources: ['SPA.3.2'], targets: ['SPA.8.2'] },
    { id: 'e31', sources: ['SPA.4.1'], targets: ['SPA.8.2'] },
    { id: 'e32', sources: ['SPA.3.2'], targets: ['SPA.8.3'] },
    { id: 'e33', sources: ['SPA.4.1'], targets: ['SPA.8.3'] },

    // Phase 9
    { id: 'e34', sources: ['SPA.3.4'], targets: ['SPA.9.1'] },
    { id: 'e35', sources: ['SPA.8.1'], targets: ['SPA.9.1'] },
    { id: 'e36', sources: ['SPA.8.2'], targets: ['SPA.9.1'] },

    // Phase 10
    { id: 'e37', sources: ['SPA.3.5'], targets: ['SPA.10.1'] },
    { id: 'e38', sources: ['SPA.4.1'], targets: ['SPA.10.1'] },
    { id: 'e39', sources: ['SPA.7.1'], targets: ['SPA.10.1'] },
    { id: 'e40', sources: ['SPA.7.2'], targets: ['SPA.10.1'] },
    { id: 'e41', sources: ['SPA.8.1'], targets: ['SPA.10.1'] },
    { id: 'e42', sources: ['SPA.8.2'], targets: ['SPA.10.1'] },
    { id: 'e43', sources: ['SPA.8.3'], targets: ['SPA.10.1'] },
    { id: 'e44', sources: ['SPA.9.1'], targets: ['SPA.10.1'] },
    { id: 'e45', sources: ['SPA.10.1'], targets: ['SPA.10.2'] },

    // Phase 11
    { id: 'e46', sources: ['SPA.10.1'], targets: ['SPA.11.1'] },
    { id: 'e47', sources: ['SPA.10.1'], targets: ['SPA.11.2'] },
    { id: 'e48', sources: ['SPA.2.5'], targets: ['SPA.11.2'], dashed: true },

    // Phase 12
    { id: 'e49', sources: ['SPA.10.2'], targets: ['SPA.12.1'] },
    { id: 'e50', sources: ['SPA.11.1'], targets: ['SPA.12.2'] },
    { id: 'e51', sources: ['SPA.11.2'], targets: ['SPA.12.2'] },
    { id: 'e52', sources: ['SPA.12.1'], targets: ['SPA.12.2'] },

    // Milestones
    { id: 'e53', sources: ['SPA.2.5'], targets: ['M1'] },
    { id: 'e54', sources: ['SPA.4.2'], targets: ['M2'] },
    { id: 'e55', sources: ['SPA.9.1'], targets: ['M3'] },
    { id: 'e56', sources: ['SPA.12.2'], targets: ['M4'] },
  ],
};

async function layout() {
  try {
    const result = await elk.layout(graph);
    generateSVG(result);
  } catch (err) {
    console.error('ELK layout error:', err);
    process.exit(1);
  }
}

function generateSVG(layoutedGraph) {
  const padding = 40;
  const width = Math.ceil(layoutedGraph.width) + padding * 2;
  const height = Math.ceil(layoutedGraph.height) + padding * 2;

  let svg = `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
  <defs>
    <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
      <polygon points="0 0, 10 3.5, 0 7" fill="#666"/>
    </marker>
    <marker id="arrowhead-dashed" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
      <polygon points="0 0, 10 3.5, 0 7" fill="#999"/>
    </marker>
    <style>
      .node-text { font-family: Helvetica, Arial, sans-serif; font-size: 10px; text-anchor: middle; }
      .milestone-text { font-family: Helvetica, Arial, sans-serif; font-size: 9px; text-anchor: middle; font-weight: bold; }
    </style>
  </defs>
  <rect width="100%" height="100%" fill="#FAFAFA"/>
  <g transform="translate(${padding}, ${padding})">
`;

  // Draw edges first (behind nodes)
  for (const edge of layoutedGraph.edges || []) {
    if (edge.sections) {
      for (const section of edge.sections) {
        const points = [section.startPoint, ...(section.bendPoints || []), section.endPoint];
        const pathData = points.map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.y}`).join(' ');
        const isDashed = graph.edges.find(e => e.id === edge.id)?.dashed;
        const style = isDashed
          ? 'stroke:#999; stroke-dasharray:5,3; fill:none; marker-end:url(#arrowhead-dashed)'
          : 'stroke:#666; fill:none; marker-end:url(#arrowhead)';
        svg += `    <path d="${pathData}" style="${style}"/>\n`;
      }
    }
  }

  // Draw nodes
  for (const node of layoutedGraph.children || []) {
    const color = COLORS[node.phase] || '#FFFFFF';
    const isMilestone = node.phase === 'milestone';
    const labelText = node.labels?.[0]?.text || node.id;
    const lines = labelText.split('\n');

    if (isMilestone) {
      // Diamond shape for milestones
      const cx = node.x + node.width / 2;
      const cy = node.y + node.height / 2;
      const rx = node.width / 2;
      const ry = node.height / 2;
      svg += `    <polygon points="${cx},${cy - ry} ${cx + rx},${cy} ${cx},${cy + ry} ${cx - rx},${cy}" fill="${color}" stroke="#999" stroke-width="1.5"/>\n`;

      // Text for milestone
      const lineHeight = 11;
      const startY = cy - ((lines.length - 1) * lineHeight) / 2;
      for (let i = 0; i < lines.length; i++) {
        svg += `    <text x="${cx}" y="${startY + i * lineHeight}" class="milestone-text">${escapeXml(lines[i])}</text>\n`;
      }
    } else {
      // Rounded rectangle for regular nodes
      svg += `    <rect x="${node.x}" y="${node.y}" width="${node.width}" height="${node.height}" rx="6" ry="6" fill="${color}" stroke="#999" stroke-width="1"/>\n`;

      // Text
      const cx = node.x + node.width / 2;
      const cy = node.y + node.height / 2;
      const lineHeight = 12;
      const startY = cy - ((lines.length - 1) * lineHeight) / 2 + 4;
      for (let i = 0; i < lines.length; i++) {
        svg += `    <text x="${cx}" y="${startY + i * lineHeight}" class="node-text">${escapeXml(lines[i])}</text>\n`;
      }
    }
  }

  svg += `  </g>
</svg>`;

  console.log(svg);
}

function escapeXml(str) {
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

layout();
