// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import { describe, it, expect, beforeEach } from "vitest";
import { SceneState } from "../adapter/SceneState";
import {
    NodeShape,
    EdgeStyle,
    hashToHex,
    type NodeDef,
    type EdgeDef,
    type LabelDef,
    type SceneOp,
} from "../types/SceneDelta";

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

describe("SceneState", () => {
    let state: SceneState;

    beforeEach(() => {
        state = new SceneState();
    });

    describe("UpsertNode", () => {
        it("creates a new node", () => {
            const node = makeNode(10, [1, 2, 3]);
            state.apply([{ op: "UpsertNode", def: node }]);

            expect(state.nodes.size).toBe(1);
            const retrieved = state.getNode(node.key);
            expect(retrieved).toBeDefined();
            expect(retrieved!.position).toEqual([1, 2, 3]);
        });

        it("replaces an existing node", () => {
            const node1 = makeNode(10, [1, 2, 3]);
            const node2 = makeNode(10, [4, 5, 6]); // Same key, different position

            state.apply([{ op: "UpsertNode", def: node1 }]);
            state.apply([{ op: "UpsertNode", def: node2 }]);

            expect(state.nodes.size).toBe(1);
            const retrieved = state.getNode(node1.key);
            expect(retrieved!.position).toEqual([4, 5, 6]);
        });
    });

    describe("RemoveNode", () => {
        it("removes a node", () => {
            const node = makeNode(10, [0, 0, 0]);
            state.apply([{ op: "UpsertNode", def: node }]);
            expect(state.nodes.size).toBe(1);

            state.apply([{ op: "RemoveNode", key: node.key }]);
            expect(state.nodes.size).toBe(0);
        });

        it("removes labels anchored to the node", () => {
            const node = makeNode(10, [0, 0, 0]);
            const label = makeLabel(20, 10, "Test Label");

            state.apply([
                { op: "UpsertNode", def: node },
                { op: "UpsertLabel", def: label },
            ]);
            expect(state.labels.size).toBe(1);

            state.apply([{ op: "RemoveNode", key: node.key }]);
            expect(state.labels.size).toBe(0);
        });
    });

    describe("UpsertEdge", () => {
        it("creates a new edge", () => {
            const edge = makeEdge(20, 10, 11);
            state.apply([{ op: "UpsertEdge", def: edge }]);

            expect(state.edges.size).toBe(1);
            const retrieved = state.getEdge(edge.key);
            expect(retrieved).toBeDefined();
        });
    });

    describe("RemoveEdge", () => {
        it("removes an edge", () => {
            const edge = makeEdge(20, 10, 11);
            state.apply([{ op: "UpsertEdge", def: edge }]);
            state.apply([{ op: "RemoveEdge", key: edge.key }]);

            expect(state.edges.size).toBe(0);
        });
    });

    describe("UpsertLabel", () => {
        it("creates a new label", () => {
            const label = makeLabel(30, 10, "Hello");
            state.apply([{ op: "UpsertLabel", def: label }]);

            expect(state.labels.size).toBe(1);
            const retrieved = state.getLabel(label.key);
            expect(retrieved).toBeDefined();
            expect(retrieved!.text).toBe("Hello");
        });
    });

    describe("RemoveLabel", () => {
        it("removes a label", () => {
            const label = makeLabel(30, 10, "Hello");
            state.apply([{ op: "UpsertLabel", def: label }]);
            state.apply([{ op: "RemoveLabel", key: label.key }]);

            expect(state.labels.size).toBe(0);
        });
    });

    describe("Clear", () => {
        it("empties all maps", () => {
            state.apply([
                { op: "UpsertNode", def: makeNode(10, [0, 0, 0]) },
                { op: "UpsertNode", def: makeNode(11, [1, 0, 0]) },
                { op: "UpsertEdge", def: makeEdge(20, 10, 11) },
                { op: "UpsertLabel", def: makeLabel(30, 10, "Test") },
            ]);

            expect(state.nodes.size).toBe(2);
            expect(state.edges.size).toBe(1);
            expect(state.labels.size).toBe(1);

            state.apply([{ op: "Clear" }]);

            expect(state.nodes.size).toBe(0);
            expect(state.edges.size).toBe(0);
            expect(state.labels.size).toBe(0);
        });
    });

    describe("isEdgeValid", () => {
        it("returns true when both endpoints exist", () => {
            const nodeA = makeNode(10, [0, 0, 0]);
            const nodeB = makeNode(11, [1, 0, 0]);
            const edge = makeEdge(20, 10, 11);

            state.apply([
                { op: "UpsertNode", def: nodeA },
                { op: "UpsertNode", def: nodeB },
                { op: "UpsertEdge", def: edge },
            ]);

            expect(state.isEdgeValid(edge)).toBe(true);
        });

        it("returns false when endpoint is missing", () => {
            const nodeA = makeNode(10, [0, 0, 0]);
            const edge = makeEdge(20, 10, 11); // nodeB (11) doesn't exist

            state.apply([
                { op: "UpsertNode", def: nodeA },
                { op: "UpsertEdge", def: edge },
            ]);

            expect(state.isEdgeValid(edge)).toBe(false);
        });
    });

    describe("batch operations", () => {
        it("applies multiple ops in order", () => {
            const ops: SceneOp[] = [
                { op: "UpsertNode", def: makeNode(10, [0, 0, 0]) },
                { op: "UpsertNode", def: makeNode(11, [1, 0, 0]) },
                { op: "UpsertEdge", def: makeEdge(20, 10, 11) },
                { op: "RemoveNode", key: makeTestHash(10) },
            ];

            state.apply(ops);

            expect(state.nodes.size).toBe(1); // Only node 11 remains
            expect(state.edges.size).toBe(1); // Edge still exists (endpoints checked at render time)
        });
    });
});
