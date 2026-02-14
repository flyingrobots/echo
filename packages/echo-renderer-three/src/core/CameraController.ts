// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * Camera controller that applies CameraState to Three.js cameras.
 *
 * Manages both perspective and orthographic cameras.
 * No time ownership - state is applied when set.
 */

import * as THREE from "three";
import type { CameraState } from "../types/CameraState";

/**
 * Camera controller managing perspective and orthographic cameras.
 */
export class CameraController {
    private perspCamera: THREE.PerspectiveCamera;
    private orthoCamera: THREE.OrthographicCamera;
    private current: THREE.Camera;

    constructor(aspect: number = 1) {
        // Initialize perspective camera
        this.perspCamera = new THREE.PerspectiveCamera(60, aspect, 0.01, 10000);

        // Initialize orthographic camera
        // Bounds will be set when apply() is called
        this.orthoCamera = new THREE.OrthographicCamera(
            -5 * aspect,
            5 * aspect,
            5,
            -5,
            0.01,
            10000
        );

        this.current = this.perspCamera;
    }

    /**
     * Apply CameraState and return the active camera.
     */
    apply(state: CameraState, aspect: number): THREE.Camera {
        if (state.projection === "Perspective") {
            // Convert radians to degrees for Three.js
            this.perspCamera.fov = (state.fovYRadians * 180) / Math.PI;
            this.perspCamera.aspect = aspect;
            this.perspCamera.near = state.near;
            this.perspCamera.far = state.far;
            this.perspCamera.position.fromArray(state.position);
            this.perspCamera.up.fromArray(state.up);
            this.perspCamera.lookAt(
                state.target[0],
                state.target[1],
                state.target[2]
            );
            this.perspCamera.updateProjectionMatrix();
            this.current = this.perspCamera;
        } else {
            const scale = state.orthoScale;
            this.orthoCamera.left = -scale * aspect;
            this.orthoCamera.right = scale * aspect;
            this.orthoCamera.top = scale;
            this.orthoCamera.bottom = -scale;
            this.orthoCamera.near = state.near;
            this.orthoCamera.far = state.far;
            this.orthoCamera.position.fromArray(state.position);
            this.orthoCamera.up.fromArray(state.up);
            this.orthoCamera.lookAt(
                state.target[0],
                state.target[1],
                state.target[2]
            );
            this.orthoCamera.updateProjectionMatrix();
            this.current = this.orthoCamera;
        }
        return this.current;
    }

    /**
     * Get the currently active camera.
     */
    get camera(): THREE.Camera {
        return this.current;
    }

    /**
     * Get the perspective camera (for direct access if needed).
     */
    get perspective(): THREE.PerspectiveCamera {
        return this.perspCamera;
    }

    /**
     * Get the orthographic camera (for direct access if needed).
     */
    get orthographic(): THREE.OrthographicCamera {
        return this.orthoCamera;
    }
}
