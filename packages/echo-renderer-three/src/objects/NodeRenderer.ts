// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * MVP node renderer using basic meshes.
 *
 * Phase 2 will add instanced rendering for performance.
 */

import * as THREE from "three";
import type { NodeDef } from "../types/SceneDelta";
import { NodeShape } from "../types/SceneDelta";

/**
 * Renders nodes as sphere or cube meshes.
 */
export class NodeRenderer {
    /** Map of hex key -> mesh */
    readonly meshes = new Map<string, THREE.Mesh>();

    // Shared geometries
    private sphereGeom = new THREE.SphereGeometry(1, 16, 16);
    private boxGeom = new THREE.BoxGeometry(1, 1, 1);

    /**
     * Sync renderer state with scene state.
     *
     * @param nodes Current node definitions (hex key -> NodeDef)
     * @param scene Three.js scene to add/remove meshes
     */
    sync(nodes: Map<string, NodeDef>, scene: THREE.Scene): void {
        // Remove deleted nodes
        for (const [key, mesh] of this.meshes) {
            if (!nodes.has(key)) {
                scene.remove(mesh);
                // Don't dispose shared geometry, just the material
                (mesh.material as THREE.Material).dispose();
                this.meshes.delete(key);
            }
        }

        // Upsert nodes
        for (const [key, def] of nodes) {
            let mesh = this.meshes.get(key);
            const needsNewMesh =
                !mesh ||
                (def.shape === NodeShape.Sphere &&
                    mesh.geometry !== this.sphereGeom) ||
                (def.shape === NodeShape.Cube && mesh.geometry !== this.boxGeom);

            if (needsNewMesh) {
                // Remove old mesh if exists
                if (mesh) {
                    scene.remove(mesh);
                    (mesh.material as THREE.Material).dispose();
                }
                // Create new mesh with correct geometry
                const geom =
                    def.shape === NodeShape.Sphere
                        ? this.sphereGeom
                        : this.boxGeom;
                const mat = new THREE.MeshBasicMaterial();
                mesh = new THREE.Mesh(geom, mat);
                mesh.userData.nodeKey = key;
                this.meshes.set(key, mesh);
                scene.add(mesh);
            }

            // At this point mesh is guaranteed to exist
            // (either from Map.get or newly created above)
            const m = mesh!;

            // Update transform and material
            m.position.fromArray(def.position);
            m.scale.setScalar(def.radius);

            const mat = m.material as THREE.MeshBasicMaterial;
            mat.color.setRGB(
                def.color[0] / 255,
                def.color[1] / 255,
                def.color[2] / 255
            );
            mat.opacity = def.color[3] / 255;
            mat.transparent = def.color[3] < 255;
        }
    }

    /**
     * Get mesh by hex key.
     */
    getMesh(keyHex: string): THREE.Mesh | undefined {
        return this.meshes.get(keyHex);
    }

    /**
     * Dispose all resources.
     */
    dispose(): void {
        for (const mesh of this.meshes.values()) {
            (mesh.material as THREE.Material).dispose();
        }
        this.meshes.clear();
        this.sphereGeom.dispose();
        this.boxGeom.dispose();
    }
}
