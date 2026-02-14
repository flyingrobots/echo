// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * MVP edge renderer using LineSegments.
 *
 * Phase 2 will add TubeGeometry for thicker edges.
 * Note: linewidth is ignored in WebGL on most platforms.
 */

import * as THREE from "three";
import type { NodeDef, EdgeDef } from "../types/SceneDelta";
import { hashToHex } from "../types/SceneDelta";

/**
 * Renders edges as line segments between nodes.
 */
export class EdgeRenderer {
    /** Map of hex key -> line */
    readonly lines = new Map<string, THREE.Line>();

    /**
     * Sync renderer state with scene state.
     *
     * @param edges Current edge definitions (hex key -> EdgeDef)
     * @param nodes Current node definitions (for endpoint positions)
     * @param scene Three.js scene to add/remove lines
     */
    sync(
        edges: Map<string, EdgeDef>,
        nodes: Map<string, NodeDef>,
        scene: THREE.Scene
    ): void {
        // Remove deleted edges
        for (const [key, line] of this.lines) {
            if (!edges.has(key)) {
                scene.remove(line);
                line.geometry.dispose();
                (line.material as THREE.Material).dispose();
                this.lines.delete(key);
            }
        }

        // Upsert edges
        for (const [key, def] of edges) {
            const nodeA = nodes.get(hashToHex(def.a));
            const nodeB = nodes.get(hashToHex(def.b));

            if (!nodeA || !nodeB) {
                // Invalid edge - hide if exists
                const existing = this.lines.get(key);
                if (existing) {
                    existing.visible = false;
                }
                continue;
            }

            let line = this.lines.get(key);
            if (!line) {
                const geom = new THREE.BufferGeometry();
                const mat = new THREE.LineBasicMaterial();
                line = new THREE.Line(geom, mat);
                line.userData.edgeKey = key;
                this.lines.set(key, line);
                scene.add(line);
            }

            // Update positions
            const positions = new Float32Array([
                ...nodeA.position,
                ...nodeB.position,
            ]);
            line.geometry.setAttribute(
                "position",
                new THREE.BufferAttribute(positions, 3)
            );
            line.geometry.computeBoundingSphere();
            line.visible = true;

            // Update material
            const mat = line.material as THREE.LineBasicMaterial;
            mat.color.setRGB(
                def.color[0] / 255,
                def.color[1] / 255,
                def.color[2] / 255
            );
            mat.opacity = def.color[3] / 255;
            mat.transparent = def.color[3] < 255;
            // Note: linewidth only works in WebGL2 with custom shader
            // Ignored in standard WebGL on most platforms
        }
    }

    /**
     * Get line by hex key.
     */
    getLine(keyHex: string): THREE.Line | undefined {
        return this.lines.get(keyHex);
    }

    /**
     * Dispose all resources.
     */
    dispose(): void {
        for (const line of this.lines.values()) {
            line.geometry.dispose();
            (line.material as THREE.Material).dispose();
        }
        this.lines.clear();
    }
}
