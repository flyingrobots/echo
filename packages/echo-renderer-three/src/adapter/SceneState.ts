// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * Pure state machine for scene management.
 *
 * Manages node/edge/label maps without any rendering dependencies.
 * Testable without Three.js or GPU.
 */

import type {
    NodeDef,
    EdgeDef,
    LabelDef,
    SceneOp,
    Hash,
} from "../types/SceneDelta";
import { hashToHex } from "../types/SceneDelta";

/**
 * Scene state: a pure data structure managing nodes, edges, and labels.
 *
 * Maps use hex strings as keys because Uint8Array can't be used as Map keys directly.
 */
export class SceneState {
    /** Current nodes in the scene. */
    readonly nodes = new Map<string, NodeDef>();
    /** Current edges in the scene. */
    readonly edges = new Map<string, EdgeDef>();
    /** Current labels in the scene. */
    readonly labels = new Map<string, LabelDef>();

    /**
     * Apply a batch of scene operations in order.
     */
    apply(ops: SceneOp[]): void {
        for (const op of ops) {
            switch (op.op) {
                case "UpsertNode":
                    this.nodes.set(hashToHex(op.def.key), op.def);
                    break;

                case "RemoveNode": {
                    const keyHex = hashToHex(op.key);
                    this.nodes.delete(keyHex);
                    // Remove labels anchored to this node
                    for (const [labelKey, label] of this.labels) {
                        if (
                            label.anchor.kind === "Node" &&
                            hashToHex(label.anchor.key) === keyHex
                        ) {
                            this.labels.delete(labelKey);
                        }
                    }
                    break;
                }

                case "UpsertEdge":
                    this.edges.set(hashToHex(op.def.key), op.def);
                    break;

                case "RemoveEdge":
                    this.edges.delete(hashToHex(op.key));
                    break;

                case "UpsertLabel":
                    this.labels.set(hashToHex(op.def.key), op.def);
                    break;

                case "RemoveLabel":
                    this.labels.delete(hashToHex(op.key));
                    break;

                case "Clear":
                    this.nodes.clear();
                    this.edges.clear();
                    this.labels.clear();
                    break;
            }
        }
    }

    /**
     * Check if an edge is valid (both endpoints exist).
     */
    isEdgeValid(edge: EdgeDef): boolean {
        return (
            this.nodes.has(hashToHex(edge.a)) &&
            this.nodes.has(hashToHex(edge.b))
        );
    }

    /**
     * Get node by key.
     */
    getNode(key: Hash): NodeDef | undefined {
        return this.nodes.get(hashToHex(key));
    }

    /**
     * Get edge by key.
     */
    getEdge(key: Hash): EdgeDef | undefined {
        return this.edges.get(hashToHex(key));
    }

    /**
     * Get label by key.
     */
    getLabel(key: Hash): LabelDef | undefined {
        return this.labels.get(hashToHex(key));
    }
}
