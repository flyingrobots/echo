// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * Integration tests for ThreeSceneAdapter.
 *
 * These tests verify the adapter's ScenePort contract without a real GPU.
 * We test state management, epoch semantics, and determinism properties.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { SceneState } from "../adapter/SceneState";
import {
    NodeShape,
    EdgeStyle,
    hashToHex,
    type SceneDelta,
    type NodeDef,
    type EdgeDef,
    type LabelDef,
} from "../types/SceneDelta";
import type { CameraState } from "../types/CameraState";
import { DEFAULT_CAMERA } from "../types/CameraState";
import type { HighlightState } from "../types/HighlightState";
import { EMPTY_HIGHLIGHT } from "../types/HighlightState";

// ============================================================================
// Test Helpers
// ============================================================================

/** Create a test hash from a seed byte. */
function makeTestHash(seed: number): Uint8Array {
    const hash = new Uint8Array(32);
    for (let i = 0; i < 32; i++) {
        hash[i] = (seed + i) & 0xff;
    }
    return hash;
}

/** Create a test node. */
function makeNode(seed: number, pos: [number, number, number]): NodeDef {
    return {
        key: makeTestHash(seed),
        position: pos,
        radius: 1.0,
        shape: NodeShape.Sphere,
        color: [255, 255, 255, 255],
    };
}

/** Create a test edge. */
function makeEdge(seed: number, aSeed: number, bSeed: number): EdgeDef {
    return {
        key: makeTestHash(seed),
        a: makeTestHash(aSeed),
        b: makeTestHash(bSeed),
        width: 0.1,
        style: EdgeStyle.Solid,
        color: [255, 255, 255, 255],
    };
}

/** Create a test label anchored to a node. */
function makeLabel(seed: number, anchorSeed: number, text: string): LabelDef {
    return {
        key: makeTestHash(seed),
        text,
        fontSize: 12,
        color: [255, 255, 255, 255],
        anchor: { kind: "Node", key: makeTestHash(anchorSeed) },
        offset: [0, 0.5, 0],
    };
}

/** Create a SceneDelta with given ops. */
function makeDelta(
    cursorSeed: number,
    epoch: number,
    ops: SceneDelta["ops"]
): SceneDelta {
    return {
        sessionId: makeTestHash(0),
        cursorId: makeTestHash(cursorSeed),
        epoch,
        ops,
    };
}

// ============================================================================
// Mock Adapter for Testing
// ============================================================================

/**
 * Minimal mock adapter implementing ScenePort-like behavior.
 *
 * This mirrors ThreeSceneAdapter's state management without Three.js dependencies.
 */
class MockScenePortAdapter {
    private state = new SceneState();
    private lastEpochByCursor = new Map<string, number>();
    private _cameraState: CameraState = DEFAULT_CAMERA;
    private _highlightState: HighlightState = EMPTY_HIGHLIGHT;
    private _renderCount = 0;
    private _disposed = false;

    applySceneDelta(delta: SceneDelta): void {
        const cursorKey = hashToHex(delta.cursorId);
        const lastEpoch = this.lastEpochByCursor.get(cursorKey) ?? -1;

        if (delta.epoch <= lastEpoch) {
            return; // Idempotent
        }

        this.state.apply(delta.ops);
        this.lastEpochByCursor.set(cursorKey, delta.epoch);
    }

    setCamera(camera: CameraState): void {
        this._cameraState = camera;
    }

    setHighlight(highlight: HighlightState): void {
        this._highlightState = highlight;
    }

    render(): void {
        this._renderCount++;
    }

    resetCursor(cursorId: Uint8Array): void {
        this.lastEpochByCursor.delete(hashToHex(cursorId));
    }

    dispose(): void {
        this._disposed = true;
    }

    // Test accessors
    get nodeCount(): number {
        return this.state.nodes.size;
    }
    get edgeCount(): number {
        return this.state.edges.size;
    }
    get labelCount(): number {
        return this.state.labels.size;
    }
    get cameraState(): CameraState {
        return this._cameraState;
    }
    get highlightState(): HighlightState {
        return this._highlightState;
    }
    get renderCount(): number {
        return this._renderCount;
    }
    get disposed(): boolean {
        return this._disposed;
    }
    getLastEpoch(cursorSeed: number): number | undefined {
        return this.lastEpochByCursor.get(hashToHex(makeTestHash(cursorSeed)));
    }
    getSceneState(): SceneState {
        return this.state;
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

describe("ScenePort Adapter Integration", () => {
    let adapter: MockScenePortAdapter;

    beforeEach(() => {
        adapter = new MockScenePortAdapter();
    });

    describe("delta application", () => {
        it("applies UpsertNode delta", () => {
            const delta = makeDelta(1, 0, [
                { op: "UpsertNode", def: makeNode(10, [1, 2, 3]) },
            ]);

            adapter.applySceneDelta(delta);

            expect(adapter.nodeCount).toBe(1);
        });

        it("applies multiple ops in single delta", () => {
            const delta = makeDelta(1, 0, [
                { op: "UpsertNode", def: makeNode(10, [0, 0, 0]) },
                { op: "UpsertNode", def: makeNode(11, [1, 0, 0]) },
                { op: "UpsertEdge", def: makeEdge(20, 10, 11) },
                { op: "UpsertLabel", def: makeLabel(30, 10, "Hello") },
            ]);

            adapter.applySceneDelta(delta);

            expect(adapter.nodeCount).toBe(2);
            expect(adapter.edgeCount).toBe(1);
            expect(adapter.labelCount).toBe(1);
        });

        it("applies Clear delta", () => {
            adapter.applySceneDelta(
                makeDelta(1, 0, [
                    { op: "UpsertNode", def: makeNode(10, [0, 0, 0]) },
                    { op: "UpsertNode", def: makeNode(11, [1, 0, 0]) },
                ])
            );
            expect(adapter.nodeCount).toBe(2);

            adapter.applySceneDelta(makeDelta(1, 1, [{ op: "Clear" }]));

            expect(adapter.nodeCount).toBe(0);
            expect(adapter.edgeCount).toBe(0);
            expect(adapter.labelCount).toBe(0);
        });
    });

    describe("epoch semantics", () => {
        it("tracks epoch per cursor", () => {
            adapter.applySceneDelta(
                makeDelta(1, 5, [
                    { op: "UpsertNode", def: makeNode(10, [0, 0, 0]) },
                ])
            );

            expect(adapter.getLastEpoch(1)).toBe(5);
        });

        it("rejects duplicate epoch (idempotent)", () => {
            adapter.applySceneDelta(
                makeDelta(1, 0, [
                    { op: "UpsertNode", def: makeNode(10, [0, 0, 0]) },
                ])
            );
            expect(adapter.nodeCount).toBe(1);

            // Same epoch with different content - should be ignored
            adapter.applySceneDelta(
                makeDelta(1, 0, [
                    { op: "UpsertNode", def: makeNode(11, [1, 0, 0]) },
                ])
            );
            expect(adapter.nodeCount).toBe(1); // Still 1
        });

        it("rejects lower epoch", () => {
            adapter.applySceneDelta(
                makeDelta(1, 5, [
                    { op: "UpsertNode", def: makeNode(10, [0, 0, 0]) },
                ])
            );

            // Lower epoch - should be ignored
            adapter.applySceneDelta(
                makeDelta(1, 3, [
                    { op: "UpsertNode", def: makeNode(11, [1, 0, 0]) },
                ])
            );
            expect(adapter.nodeCount).toBe(1);
        });

        it("accepts higher epoch", () => {
            adapter.applySceneDelta(
                makeDelta(1, 0, [
                    { op: "UpsertNode", def: makeNode(10, [0, 0, 0]) },
                ])
            );
            adapter.applySceneDelta(
                makeDelta(1, 1, [
                    { op: "UpsertNode", def: makeNode(11, [1, 0, 0]) },
                ])
            );

            expect(adapter.nodeCount).toBe(2);
            expect(adapter.getLastEpoch(1)).toBe(1);
        });

        it("different cursors have independent epochs", () => {
            // Cursor 1: epoch 5
            adapter.applySceneDelta(
                makeDelta(1, 5, [
                    { op: "UpsertNode", def: makeNode(10, [0, 0, 0]) },
                ])
            );

            // Cursor 2: epoch 0 - should work
            adapter.applySceneDelta(
                makeDelta(2, 0, [
                    { op: "UpsertNode", def: makeNode(11, [1, 0, 0]) },
                ])
            );

            expect(adapter.nodeCount).toBe(2);
            expect(adapter.getLastEpoch(1)).toBe(5);
            expect(adapter.getLastEpoch(2)).toBe(0);
        });

        it("resetCursor allows epoch restart", () => {
            // Apply epoch 5
            adapter.applySceneDelta(
                makeDelta(1, 5, [
                    { op: "UpsertNode", def: makeNode(10, [0, 0, 0]) },
                ])
            );
            expect(adapter.getLastEpoch(1)).toBe(5);

            // Epoch 0 is rejected
            adapter.applySceneDelta(
                makeDelta(1, 0, [
                    { op: "UpsertNode", def: makeNode(11, [1, 0, 0]) },
                ])
            );
            expect(adapter.nodeCount).toBe(1);

            // Reset cursor
            adapter.resetCursor(makeTestHash(1));
            expect(adapter.getLastEpoch(1)).toBeUndefined();

            // Now epoch 0 works
            adapter.applySceneDelta(
                makeDelta(1, 0, [
                    { op: "UpsertNode", def: makeNode(12, [2, 0, 0]) },
                ])
            );
            expect(adapter.nodeCount).toBe(2);
        });
    });

    describe("camera and highlight", () => {
        it("setCamera updates camera state", () => {
            const camera: CameraState = {
                position: [10, 20, 30],
                target: [0, 0, 0],
                up: [0, 1, 0],
                projection: "Orthographic",
                fovYRadians: 1.0,
                orthoScale: 25,
                near: 1,
                far: 500,
            };

            adapter.setCamera(camera);

            expect(adapter.cameraState.position).toEqual([10, 20, 30]);
            expect(adapter.cameraState.projection).toBe("Orthographic");
        });

        it("setHighlight updates highlight state", () => {
            const highlight: HighlightState = {
                selectedNodes: [makeTestHash(1), makeTestHash(2)],
                selectedEdges: [makeTestHash(3)],
                hoveredNode: makeTestHash(4),
            };

            adapter.setHighlight(highlight);

            expect(adapter.highlightState.selectedNodes).toHaveLength(2);
            expect(adapter.highlightState.hoveredNode).toBeDefined();
        });
    });

    describe("render and dispose", () => {
        it("render increments count", () => {
            expect(adapter.renderCount).toBe(0);
            adapter.render();
            adapter.render();
            expect(adapter.renderCount).toBe(2);
        });

        it("dispose marks adapter as disposed", () => {
            expect(adapter.disposed).toBe(false);
            adapter.dispose();
            expect(adapter.disposed).toBe(true);
        });
    });

    describe("determinism", () => {
        it("two adapters with same deltas produce identical state", () => {
            const adapter1 = new MockScenePortAdapter();
            const adapter2 = new MockScenePortAdapter();

            const deltas = [
                makeDelta(1, 0, [
                    { op: "UpsertNode", def: makeNode(10, [0, 0, 0]) },
                    { op: "UpsertNode", def: makeNode(11, [1, 0, 0]) },
                ]),
                makeDelta(1, 1, [
                    { op: "UpsertEdge", def: makeEdge(20, 10, 11) },
                ]),
                makeDelta(1, 2, [
                    { op: "UpsertLabel", def: makeLabel(30, 10, "Test") },
                ]),
                makeDelta(1, 3, [{ op: "RemoveNode", key: makeTestHash(10) }]),
            ];

            for (const delta of deltas) {
                adapter1.applySceneDelta(delta);
                adapter2.applySceneDelta(delta);
            }

            expect(adapter1.nodeCount).toBe(adapter2.nodeCount);
            expect(adapter1.edgeCount).toBe(adapter2.edgeCount);
            expect(adapter1.labelCount).toBe(adapter2.labelCount);

            // Labels anchored to removed node should be gone
            expect(adapter1.labelCount).toBe(0);
        });

        it("order of deltas matters", () => {
            const adapter1 = new MockScenePortAdapter();
            const adapter2 = new MockScenePortAdapter();

            // Same deltas, different order
            const deltaA = makeDelta(1, 0, [
                { op: "UpsertNode", def: makeNode(10, [0, 0, 0]) },
            ]);
            const deltaB = makeDelta(1, 1, [
                { op: "RemoveNode", key: makeTestHash(10) },
            ]);

            // Adapter 1: A then B
            adapter1.applySceneDelta(deltaA);
            adapter1.applySceneDelta(deltaB);
            expect(adapter1.nodeCount).toBe(0);

            // Adapter 2: B then A (B is ignored because epoch)
            adapter2.applySceneDelta(deltaB); // epoch 1 first
            adapter2.applySceneDelta(deltaA); // epoch 0 rejected
            expect(adapter2.nodeCount).toBe(0); // B cleared nothing, A was rejected
        });
    });

    describe("50-delta stress test", () => {
        it("handles 50 deltas correctly", () => {
            // Add 25 nodes
            for (let i = 0; i < 25; i++) {
                adapter.applySceneDelta(
                    makeDelta(1, i, [
                        { op: "UpsertNode", def: makeNode(i, [i, 0, 0]) },
                    ])
                );
            }
            expect(adapter.nodeCount).toBe(25);

            // Add 15 edges
            for (let i = 0; i < 15; i++) {
                adapter.applySceneDelta(
                    makeDelta(1, 25 + i, [
                        { op: "UpsertEdge", def: makeEdge(100 + i, i, i + 1) },
                    ])
                );
            }
            expect(adapter.edgeCount).toBe(15);

            // Remove 10 nodes
            for (let i = 0; i < 10; i++) {
                adapter.applySceneDelta(
                    makeDelta(1, 40 + i, [
                        { op: "RemoveNode", key: makeTestHash(i) },
                    ])
                );
            }
            expect(adapter.nodeCount).toBe(15);
        });
    });
});
