// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * Camera state types mirroring Rust echo-scene-port.
 */

/** Camera projection type. */
export type ProjectionKind = "Perspective" | "Orthographic";

/** Camera state for rendering. */
export interface CameraState {
    position: [number, number, number];
    target: [number, number, number];
    up: [number, number, number];
    projection: ProjectionKind;
    /** Field of view in radians (perspective only). */
    fovYRadians: number;
    /** Vertical extent in world units (orthographic only). */
    orthoScale: number;
    near: number;
    far: number;
}

/** Default camera state (60° FOV perspective). */
export const DEFAULT_CAMERA: CameraState = {
    position: [0, 0, 5],
    target: [0, 0, 0],
    up: [0, 1, 0],
    projection: "Perspective",
    fovYRadians: Math.PI / 3, // 60 degrees
    orthoScale: 10,
    near: 0.01,
    far: 10000,
};
