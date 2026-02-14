// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * Type exports for echo-renderer-three.
 *
 * Domain types mirror Rust echo-scene-port exactly.
 * Adapter-local types (RenderContext, Profiler) are NOT in the Rust crate.
 */

// Domain types (mirror Rust)
export * from "./SceneDelta";
export * from "./CameraState";
export * from "./HighlightState";

// ============================================================================
// Adapter-local types (NOT in Rust port crate)
// ============================================================================

import type { SceneDelta, Hash } from "./SceneDelta";
import type { CameraState } from "./CameraState";
import type { HighlightState } from "./HighlightState";

/** Render timing context. Adapter-local, not part of domain contract. */
export interface RenderContext {
    /** Monotonic frame counter from app. */
    frameIndex: number;
    /** App-controlled time in seconds. */
    timeSeconds: number;
    /** Delta time since last frame. */
    dtSeconds: number;
    /** Viewport width in pixels. */
    width: number;
    /** Viewport height in pixels. */
    height: number;
    /** Device pixel ratio. */
    dpr: number;
}

/** Optional profiler interface. Inject to enable timing. */
export interface Profiler {
    markStart(label: string): void;
    markEnd(label: string): number; // returns ms
}

/** No-op profiler for production. */
export const NULL_PROFILER: Profiler = {
    markStart: () => {},
    markEnd: () => 0,
};

/** ScenePort interface (TypeScript version). */
export interface ScenePort {
    /** Apply a scene delta. Idempotent per (cursorId, epoch). */
    applySceneDelta(delta: SceneDelta): void;
    /** Set camera state. */
    setCamera(camera: CameraState): void;
    /** Set highlight state (selection/hover). */
    setHighlight(highlight: HighlightState): void;
    /** Render the current scene. */
    render(ctx: RenderContext): void;
    /** Resize viewport. */
    resize(width: number, height: number, dpr: number): void;
    /** Reset epoch tracking only. Scene state is NOT cleared. */
    resetCursor(cursorId: Hash): void;
    /** Dispose all resources. */
    dispose(): void;
}
