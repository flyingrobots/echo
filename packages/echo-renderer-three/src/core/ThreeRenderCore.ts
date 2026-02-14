// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * Minimal WebGLRenderer wrapper.
 *
 * - No RAF ownership
 * - No timing calls
 * - No task scheduler
 *
 * The adapter calls render() when the app decides to render.
 */

import * as THREE from "three";

/** Configuration options for ThreeRenderCore. */
export interface RenderCoreOptions {
    /** Enable antialiasing (default: true). */
    antialias?: boolean;
    /** Device pixel ratio (default: 1). */
    pixelRatio?: number;
    /** Power preference hint. */
    powerPreference?: "default" | "high-performance" | "low-power";
}

/**
 * WebGL renderer wrapper.
 *
 * Owns the WebGLRenderer instance. Does NOT own the render loop.
 */
export class ThreeRenderCore {
    readonly renderer: THREE.WebGLRenderer;

    constructor(canvas: HTMLCanvasElement, options: RenderCoreOptions = {}) {
        this.renderer = new THREE.WebGLRenderer({
            canvas,
            antialias: options.antialias ?? true,
            powerPreference: options.powerPreference ?? "high-performance",
            // Preserve drawing buffer for potential screenshot support
            preserveDrawingBuffer: false,
        });
        this.renderer.setPixelRatio(options.pixelRatio ?? 1);
        // Linear color space for modern rendering
        this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    }

    /**
     * Render a scene with a camera.
     *
     * No timing calls, no RAF - just render.
     */
    render(scene: THREE.Scene, camera: THREE.Camera): void {
        this.renderer.render(scene, camera);
    }

    /**
     * Resize the renderer viewport.
     */
    resize(width: number, height: number, dpr: number): void {
        this.renderer.setSize(width, height, false);
        this.renderer.setPixelRatio(dpr);
    }

    /**
     * Get current viewport size.
     */
    getSize(): { width: number; height: number } {
        const size = new THREE.Vector2();
        this.renderer.getSize(size);
        return { width: size.x, height: size.y };
    }

    /**
     * Dispose the renderer.
     */
    dispose(): void {
        this.renderer.dispose();
    }
}
