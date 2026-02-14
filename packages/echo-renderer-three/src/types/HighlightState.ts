// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * Highlight state types mirroring Rust echo-scene-port.
 */

import type { NodeKey, EdgeKey } from "./SceneDelta";

/** Highlight state for selection/hover feedback. */
export interface HighlightState {
    selectedNodes: NodeKey[];
    selectedEdges: EdgeKey[];
    hoveredNode?: NodeKey;
    hoveredEdge?: EdgeKey;
}

/** Empty highlight state constant. */
export const EMPTY_HIGHLIGHT: HighlightState = {
    selectedNodes: [],
    selectedEdges: [],
};
