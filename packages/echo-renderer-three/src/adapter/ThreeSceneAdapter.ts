// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * ThreeSceneAdapter: The main ScenePort implementation for Three.js.
 *
 * Wires together all components:
 * - SceneState for delta application
 * - ThreeRenderCore for WebGL rendering
 * - CameraController for camera management
 * - NodeRenderer, EdgeRenderer, LabelRenderer for objects
 * - HighlightRenderer for selection/hover feedback
 */

import * as THREE from "three";
import type {
    ScenePort,
    RenderContext,
    Profiler,
    Hash,
} from "../types";
import {
    NULL_PROFILER,
    hashToHex,
} from "../types";
import type { SceneDelta } from "../types/SceneDelta";
import type { CameraState } from "../types/CameraState";
import { DEFAULT_CAMERA } from "../types/CameraState";
import type { HighlightState } from "../types/HighlightState";
import { EMPTY_HIGHLIGHT } from "../types/HighlightState";
import { SceneState } from "./SceneState";
import { ThreeRenderCore, type RenderCoreOptions } from "../core/ThreeRenderCore";
import { CameraController } from "../core/CameraController";
import { NodeRenderer } from "../objects/NodeRenderer";
import { EdgeRenderer } from "../objects/EdgeRenderer";
import { LabelRenderer } from "../objects/LabelRenderer";
import { HighlightRenderer } from "../objects/HighlightRenderer";

/** Configuration options for ThreeSceneAdapter. */
export interface ThreeSceneAdapterOptions extends RenderCoreOptions {
    /** Profiler for performance measurement. */
    profiler?: Profiler;
    /** Background color (default: transparent). */
    backgroundColor?: number;
    /** Background alpha (default: 0). */
    backgroundAlpha?: number;
}

/**
 * Three.js implementation of ScenePort.
 *
 * Receives deltas, manages scene state, and renders on demand.
 * No time ownership - app controls when to render.
 */
export class ThreeSceneAdapter implements ScenePort {
    // State management
    private state = new SceneState();
    private lastEpochByCursor = new Map<string, number>();

    // Rendering
    private core: ThreeRenderCore;
    private cameraController: CameraController;
    private scene = new THREE.Scene();

    // Object renderers
    private nodeRenderer = new NodeRenderer();
    private edgeRenderer = new EdgeRenderer();
    private labelRenderer = new LabelRenderer();
    private highlightRenderer = new HighlightRenderer();

    // Current state
    private currentHighlight: HighlightState = EMPTY_HIGHLIGHT;
    private cameraState: CameraState = DEFAULT_CAMERA;
    private profiler: Profiler;

    // Track if scene needs re-sync
    private dirty = false;

    constructor(
        canvas: HTMLCanvasElement,
        options: ThreeSceneAdapterOptions = {}
    ) {
        this.core = new ThreeRenderCore(canvas, options);
        this.cameraController = new CameraController(
            canvas.width / canvas.height
        );
        this.profiler = options.profiler ?? NULL_PROFILER;

        // Set background
        if (options.backgroundColor !== undefined) {
            this.scene.background = new THREE.Color(options.backgroundColor);
        }
    }

    /**
     * Apply a scene delta.
     *
     * Idempotent per (cursorId, epoch).
     */
    applySceneDelta(delta: SceneDelta): void {
        const cursorKey = hashToHex(delta.cursorId);
        const lastEpoch = this.lastEpochByCursor.get(cursorKey) ?? -1;

        if (delta.epoch <= lastEpoch) {
            // Already processed or stale
            return;
        }

        this.profiler.markStart("applyDelta");
        this.state.apply(delta.ops);
        this.lastEpochByCursor.set(cursorKey, delta.epoch);
        this.dirty = true;
        this.profiler.markEnd("applyDelta");
    }

    /**
     * Set camera state.
     */
    setCamera(camera: CameraState): void {
        this.cameraState = camera;
    }

    /**
     * Set highlight state (selection/hover).
     */
    setHighlight(highlight: HighlightState): void {
        this.currentHighlight = highlight;
    }

    /**
     * Render the current scene.
     */
    render(ctx: RenderContext): void {
        this.profiler.markStart("render");

        // Sync Three.js objects with scene state if dirty
        if (this.dirty) {
            this.profiler.markStart("syncObjects");
            this.syncObjects();
            this.dirty = false;
            this.profiler.markEnd("syncObjects");
        }

        // Apply highlights
        this.profiler.markStart("highlight");
        this.highlightRenderer.apply(
            this.currentHighlight,
            this.nodeRenderer.meshes,
            this.edgeRenderer.lines
        );
        this.profiler.markEnd("highlight");

        // Update camera
        const aspect = ctx.width / ctx.height;
        const camera = this.cameraController.apply(this.cameraState, aspect);

        // Render
        this.profiler.markStart("draw");
        this.core.render(this.scene, camera);
        this.profiler.markEnd("draw");

        this.profiler.markEnd("render");
    }

    /**
     * Resize viewport.
     */
    resize(width: number, height: number, dpr: number): void {
        this.core.resize(width, height, dpr);
    }

    /**
     * Reset epoch tracking for a cursor.
     *
     * This ONLY clears epoch tracking. Scene state is NOT cleared.
     * Use SceneOp.Clear to clear the scene.
     */
    resetCursor(cursorId: Hash): void {
        this.lastEpochByCursor.delete(hashToHex(cursorId));
    }

    /**
     * Dispose all resources.
     */
    dispose(): void {
        this.nodeRenderer.dispose();
        this.edgeRenderer.dispose();
        this.labelRenderer.dispose();
        this.core.dispose();
    }

    // ========================================================================
    // Accessors for testing/debugging
    // ========================================================================

    /** Get current node count. */
    get nodeCount(): number {
        return this.state.nodes.size;
    }

    /** Get current edge count. */
    get edgeCount(): number {
        return this.state.edges.size;
    }

    /** Get current label count. */
    get labelCount(): number {
        return this.state.labels.size;
    }

    /** Get the Three.js scene (for debugging). */
    get threeScene(): THREE.Scene {
        return this.scene;
    }

    /** Get the scene state (for testing). */
    get sceneState(): SceneState {
        return this.state;
    }

    // ========================================================================
    // Private
    // ========================================================================

    /**
     * Sync Three.js objects with scene state.
     */
    private syncObjects(): void {
        this.nodeRenderer.sync(this.state.nodes, this.scene);
        this.edgeRenderer.sync(this.state.edges, this.state.nodes, this.scene);
        this.labelRenderer.sync(this.state.labels, this.state.nodes, this.scene);
    }
}
