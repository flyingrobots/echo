// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * Scene delta types mirroring Rust echo-scene-port.
 *
 * These types define the domain contract between TTD Controller and renderers.
 */

/** 32-byte key as Uint8Array. Matches Rust [u8; 32]. */
export type Hash = Uint8Array; // length 32
export type NodeKey = Hash;
export type EdgeKey = Hash;
export type LabelKey = Hash;

/** Convert Hash to hex string for use as Map key. */
export function hashToHex(h: Hash): string {
    return Array.from(h)
        .map((b) => b.toString(16).padStart(2, "0"))
        .join("");
}

/** Parse hex string back to Hash. */
export function hexToHash(hex: string): Hash {
    if (hex.length !== 64) {
        throw new Error(`Invalid hash hex length: expected 64, got ${hex.length}`);
    }
    const bytes = new Uint8Array(32);
    for (let i = 0; i < 32; i++) {
        bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
    }
    return bytes;
}

/** Node shape for rendering. */
export const NodeShape = {
    Sphere: 0,
    Cube: 1,
} as const;
export type NodeShape = (typeof NodeShape)[keyof typeof NodeShape];

/** Edge visual style. */
export const EdgeStyle = {
    Solid: 0,
    Dashed: 1,
} as const;
export type EdgeStyle = (typeof EdgeStyle)[keyof typeof EdgeStyle];

/** RGBA color as [r, g, b, a] where each is 0-255. */
export type ColorRgba8 = [number, number, number, number];

/** Node definition. */
export interface NodeDef {
    key: NodeKey;
    position: [number, number, number];
    radius: number;
    shape: NodeShape;
    color: ColorRgba8;
}

/** Edge definition connecting two nodes. */
export interface EdgeDef {
    key: EdgeKey;
    a: NodeKey;
    b: NodeKey;
    width: number;
    style: EdgeStyle;
    color: ColorRgba8;
}

/** Label anchor type. */
export type LabelAnchor =
    | { kind: "Node"; key: NodeKey }
    | { kind: "World"; position: [number, number, number] };

/** Label definition for text overlays. */
export interface LabelDef {
    key: LabelKey;
    text: string;
    fontSize: number;
    color: ColorRgba8;
    anchor: LabelAnchor;
    offset: [number, number, number];
}

/** MVP scene operations. */
export type SceneOp =
    | { op: "UpsertNode"; def: NodeDef }
    | { op: "RemoveNode"; key: NodeKey }
    | { op: "UpsertEdge"; def: EdgeDef }
    | { op: "RemoveEdge"; key: EdgeKey }
    | { op: "UpsertLabel"; def: LabelDef }
    | { op: "RemoveLabel"; key: LabelKey }
    | { op: "Clear" };

/** Scene delta: a batch of operations scoped to a cursor epoch. */
export interface SceneDelta {
    sessionId: Hash;
    cursorId: Hash;
    epoch: number;
    ops: SceneOp[];
}
