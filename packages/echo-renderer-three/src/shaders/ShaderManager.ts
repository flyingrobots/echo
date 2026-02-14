// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * Shader chunk registry with #include preprocessing.
 *
 * Simplified from the original: no uTime uniform, no film grain.
 * All time-dependent effects require explicit seed input.
 */

/** Registered shader chunks. */
const DEFAULT_CHUNKS: Record<string, string> = {
    noise: `
// Simple hash-based noise (deterministic)
float hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);

    float a = hash(i);
    float b = hash(i + vec2(1.0, 0.0));
    float c = hash(i + vec2(0.0, 1.0));
    float d = hash(i + vec2(1.0, 1.0));

    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}
`,
    fresnel: `
// Schlick's Fresnel approximation
float fresnel(vec3 viewDir, vec3 normal, float power) {
    return pow(1.0 - max(dot(viewDir, normal), 0.0), power);
}
`,
};

/**
 * Shader manager for GLSL chunk composition.
 *
 * No singleton pattern - create one per adapter.
 */
export class ShaderManager {
    private chunks: Map<string, string>;

    constructor() {
        this.chunks = new Map(Object.entries(DEFAULT_CHUNKS));
    }

    /**
     * Register a shader chunk.
     */
    registerChunk(name: string, source: string): void {
        this.chunks.set(name, source);
    }

    /**
     * Get a registered chunk.
     */
    getChunk(name: string): string | undefined {
        return this.chunks.get(name);
    }

    /**
     * Process shader source, replacing #include <name> with chunk content.
     */
    processIncludes(source: string): string {
        return source.replace(/#include\s+<(\w+)>/g, (_, name: string) => {
            const chunk = this.chunks.get(name);
            if (!chunk) {
                console.warn(`ShaderManager: unknown chunk "${name}"`);
                return `// Missing chunk: ${name}`;
            }
            return chunk;
        });
    }
}
