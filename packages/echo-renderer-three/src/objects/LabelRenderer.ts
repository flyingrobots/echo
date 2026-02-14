// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * MVP label renderer using CanvasTexture sprites.
 *
 * Phase 2 will add SDF font rendering for better quality.
 */

import * as THREE from "three";
import type { NodeDef, LabelDef } from "../types/SceneDelta";
import { hashToHex } from "../types/SceneDelta";

/** Canvas font size for label rendering. */
const CANVAS_FONT_SIZE = 64;
/** Font family for labels. */
const FONT_FAMILY = "sans-serif";
/** Padding around label text. */
const PADDING = 8;

/**
 * Renders labels as billboard sprites with canvas textures.
 */
export class LabelRenderer {
    /** Map of hex key -> sprite */
    private sprites = new Map<string, THREE.Sprite>();
    /** Cached label text for change detection */
    private labelCache = new Map<string, string>();

    /**
     * Sync renderer state with scene state.
     *
     * @param labels Current label definitions (hex key -> LabelDef)
     * @param nodes Current node definitions (for anchor positions)
     * @param scene Three.js scene to add/remove sprites
     */
    sync(
        labels: Map<string, LabelDef>,
        nodes: Map<string, NodeDef>,
        scene: THREE.Scene
    ): void {
        // Remove deleted labels
        for (const [key, sprite] of this.sprites) {
            if (!labels.has(key)) {
                scene.remove(sprite);
                const mat = sprite.material as THREE.SpriteMaterial;
                mat.map?.dispose();
                mat.dispose();
                this.sprites.delete(key);
                this.labelCache.delete(key);
            }
        }

        // Upsert labels
        for (const [key, def] of labels) {
            // Calculate position from anchor
            let position: [number, number, number];
            if (def.anchor.kind === "Node") {
                const node = nodes.get(hashToHex(def.anchor.key));
                if (!node) {
                    // Anchor missing - hide sprite if exists
                    const existing = this.sprites.get(key);
                    if (existing) {
                        existing.visible = false;
                    }
                    continue;
                }
                position = [
                    node.position[0] + def.offset[0],
                    node.position[1] + def.offset[1],
                    node.position[2] + def.offset[2],
                ];
            } else {
                position = [
                    def.anchor.position[0] + def.offset[0],
                    def.anchor.position[1] + def.offset[1],
                    def.anchor.position[2] + def.offset[2],
                ];
            }

            let sprite = this.sprites.get(key);
            const cachedText = this.labelCache.get(key);
            const textChanged =
                cachedText !== this.getLabelCacheKey(def);

            if (!sprite) {
                // Create new sprite
                const canvas = this.createLabelCanvas(def);
                const texture = new THREE.CanvasTexture(canvas);
                texture.minFilter = THREE.LinearFilter;
                texture.magFilter = THREE.LinearFilter;
                const mat = new THREE.SpriteMaterial({
                    map: texture,
                    transparent: true,
                    depthWrite: false,
                });
                sprite = new THREE.Sprite(mat);
                sprite.userData.labelKey = key;
                this.sprites.set(key, sprite);
                this.labelCache.set(key, this.getLabelCacheKey(def));
                scene.add(sprite);
            } else if (textChanged) {
                // Update texture
                const canvas = this.createLabelCanvas(def);
                const mat = sprite.material as THREE.SpriteMaterial;
                mat.map?.dispose();
                mat.map = new THREE.CanvasTexture(canvas);
                mat.map.minFilter = THREE.LinearFilter;
                mat.map.magFilter = THREE.LinearFilter;
                this.labelCache.set(key, this.getLabelCacheKey(def));
            }

            sprite.visible = true;
            sprite.position.fromArray(position);

            // Scale sprite based on font size and text length
            // The aspect ratio comes from the canvas dimensions
            const canvas = (sprite.material as THREE.SpriteMaterial).map
                ?.image as HTMLCanvasElement | undefined;
            if (canvas) {
                const aspect = canvas.width / canvas.height;
                const scale = def.fontSize;
                sprite.scale.set(scale * aspect, scale, 1);
            }
        }
    }

    /**
     * Create a canvas with label text.
     */
    private createLabelCanvas(def: LabelDef): HTMLCanvasElement {
        const canvas = document.createElement("canvas");
        const ctx = canvas.getContext("2d")!;

        // Measure text
        ctx.font = `${CANVAS_FONT_SIZE}px ${FONT_FAMILY}`;
        const metrics = ctx.measureText(def.text);
        const textWidth = Math.ceil(metrics.width);
        const textHeight = CANVAS_FONT_SIZE;

        // Size canvas
        canvas.width = textWidth + PADDING * 2;
        canvas.height = textHeight + PADDING * 2;

        // Fill background (optional - transparent by default)
        // ctx.fillStyle = 'rgba(0, 0, 0, 0.5)';
        // ctx.fillRect(0, 0, canvas.width, canvas.height);

        // Draw text
        ctx.font = `${CANVAS_FONT_SIZE}px ${FONT_FAMILY}`;
        ctx.textBaseline = "middle";
        ctx.textAlign = "center";
        ctx.fillStyle = `rgba(${def.color[0]}, ${def.color[1]}, ${def.color[2]}, ${def.color[3] / 255})`;
        ctx.fillText(def.text, canvas.width / 2, canvas.height / 2);

        return canvas;
    }

    /**
     * Get cache key for label change detection.
     */
    private getLabelCacheKey(def: LabelDef): string {
        return `${def.text}|${def.fontSize}|${def.color.join(",")}`;
    }

    /**
     * Get sprite by hex key.
     */
    getSprite(keyHex: string): THREE.Sprite | undefined {
        return this.sprites.get(keyHex);
    }

    /**
     * Dispose all resources.
     */
    dispose(): void {
        for (const sprite of this.sprites.values()) {
            const mat = sprite.material as THREE.SpriteMaterial;
            mat.map?.dispose();
            mat.dispose();
        }
        this.sprites.clear();
        this.labelCache.clear();
    }
}
