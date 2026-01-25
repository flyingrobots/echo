// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * echo-renderer-three: Three.js ScenePort adapter for Echo TTD.
 *
 * This package provides a hexagonal rendering adapter that receives
 * SceneDelta messages and renders them using Three.js.
 *
 * @example
 * ```typescript
 * import { ThreeSceneAdapter } from 'echo-renderer-three';
 *
 * const canvas = document.getElementById('canvas') as HTMLCanvasElement;
 * const adapter = new ThreeSceneAdapter(canvas);
 *
 * // Apply deltas from TTD Controller
 * adapter.applySceneDelta(delta);
 *
 * // Render when ready
 * adapter.render({
 *     frameIndex: 0,
 *     timeSeconds: 0,
 *     dtSeconds: 0.016,
 *     width: canvas.width,
 *     height: canvas.height,
 *     dpr: window.devicePixelRatio,
 * });
 * ```
 */

// Types (domain contract)
export * from "./types";

// Adapter
export { ThreeSceneAdapter, type ThreeSceneAdapterOptions } from "./adapter/ThreeSceneAdapter";
export { SceneState } from "./adapter/SceneState";

// Core components (for advanced use)
export { ThreeRenderCore, type RenderCoreOptions } from "./core/ThreeRenderCore";
export { CameraController } from "./core/CameraController";

// Object renderers (for advanced use)
export { NodeRenderer } from "./objects/NodeRenderer";
export { EdgeRenderer } from "./objects/EdgeRenderer";
export { LabelRenderer } from "./objects/LabelRenderer";
export { HighlightRenderer } from "./objects/HighlightRenderer";

// Utilities
export { ShaderManager } from "./shaders/ShaderManager";
export { AssetManager } from "./assets/AssetManager";
