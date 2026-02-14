// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * URL-keyed texture cache.
 *
 * No singleton pattern - create one per adapter instance.
 * Content-addressed assets are deferred to Phase 2.
 */

import * as THREE from "three";

/**
 * Asset manager for texture loading and caching.
 *
 * Each adapter instance should have its own AssetManager.
 */
export class AssetManager {
    private textures = new Map<string, THREE.Texture>();
    private loadingManager: THREE.LoadingManager;

    constructor(loadingManager?: THREE.LoadingManager) {
        this.loadingManager = loadingManager ?? new THREE.LoadingManager();
    }

    /**
     * Get or load a texture by URL.
     *
     * Returns cached texture if already loaded.
     */
    getTexture(url: string): THREE.Texture {
        let tex = this.textures.get(url);
        if (!tex) {
            tex = new THREE.TextureLoader(this.loadingManager).load(url);
            this.textures.set(url, tex);
        }
        return tex;
    }

    /**
     * Check if a texture is cached.
     */
    hasTexture(url: string): boolean {
        return this.textures.has(url);
    }

    /**
     * Get the number of cached textures.
     */
    get textureCount(): number {
        return this.textures.size;
    }

    /**
     * Dispose all cached textures.
     */
    dispose(): void {
        for (const tex of this.textures.values()) {
            tex.dispose();
        }
        this.textures.clear();
    }
}
