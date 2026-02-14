// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * MVP highlight renderer using color tint.
 *
 * Phase 2 will add outline/stencil effects.
 */

import * as THREE from "three";
import type { HighlightState } from "../types/HighlightState";
import { hashToHex } from "../types/SceneDelta";

/** Selection tint color (yellow). */
const SELECTED_COLOR = new THREE.Color(0xffff00);
/** Hover tint color (cyan). */
const HOVERED_COLOR = new THREE.Color(0x00ffff);
/** Tint blend factor. */
const TINT_FACTOR = 0.5;

/**
 * Applies highlight effects to nodes and edges via color tinting.
 */
export class HighlightRenderer {
    /** Original colors for restoration. */
    private originalColors = new Map<string, THREE.Color>();

    /**
     * Apply highlight state to meshes and lines.
     *
     * @param highlight Current highlight state
     * @param nodeMeshes Node meshes by hex key
     * @param edgeLines Edge lines by hex key
     */
    apply(
        highlight: HighlightState,
        nodeMeshes: Map<string, THREE.Mesh>,
        edgeLines: Map<string, THREE.Line>
    ): void {
        // Reset all to original colors
        for (const [key, originalColor] of this.originalColors) {
            const mesh = nodeMeshes.get(key);
            const line = edgeLines.get(key);
            const obj = mesh ?? line;
            if (obj) {
                const mat = obj.material as
                    | THREE.MeshBasicMaterial
                    | THREE.LineBasicMaterial;
                mat.color.copy(originalColor);
            }
        }
        this.originalColors.clear();

        // Apply selection tint to nodes
        for (const key of highlight.selectedNodes) {
            const keyHex = hashToHex(key);
            const mesh = nodeMeshes.get(keyHex);
            if (mesh) {
                const mat = mesh.material as THREE.MeshBasicMaterial;
                this.originalColors.set(keyHex, mat.color.clone());
                mat.color.lerp(SELECTED_COLOR, TINT_FACTOR);
            }
        }

        // Apply selection tint to edges
        for (const key of highlight.selectedEdges) {
            const keyHex = hashToHex(key);
            const line = edgeLines.get(keyHex);
            if (line) {
                const mat = line.material as THREE.LineBasicMaterial;
                this.originalColors.set(keyHex, mat.color.clone());
                mat.color.lerp(SELECTED_COLOR, TINT_FACTOR);
            }
        }

        // Apply hover tint to node (overwrites selection if both)
        if (highlight.hoveredNode) {
            const keyHex = hashToHex(highlight.hoveredNode);
            const mesh = nodeMeshes.get(keyHex);
            if (mesh) {
                const mat = mesh.material as THREE.MeshBasicMaterial;
                if (!this.originalColors.has(keyHex)) {
                    this.originalColors.set(keyHex, mat.color.clone());
                }
                mat.color.lerp(HOVERED_COLOR, TINT_FACTOR);
            }
        }

        // Apply hover tint to edge (overwrites selection if both)
        if (highlight.hoveredEdge) {
            const keyHex = hashToHex(highlight.hoveredEdge);
            const line = edgeLines.get(keyHex);
            if (line) {
                const mat = line.material as THREE.LineBasicMaterial;
                if (!this.originalColors.has(keyHex)) {
                    this.originalColors.set(keyHex, mat.color.clone());
                }
                mat.color.lerp(HOVERED_COLOR, TINT_FACTOR);
            }
        }
    }

    /**
     * Clear all highlights and restore original colors.
     */
    clear(
        nodeMeshes: Map<string, THREE.Mesh>,
        edgeLines: Map<string, THREE.Line>
    ): void {
        for (const [key, originalColor] of this.originalColors) {
            const mesh = nodeMeshes.get(key);
            const line = edgeLines.get(key);
            const obj = mesh ?? line;
            if (obj) {
                const mat = obj.material as
                    | THREE.MeshBasicMaterial
                    | THREE.LineBasicMaterial;
                mat.color.copy(originalColor);
            }
        }
        this.originalColors.clear();
    }
}
