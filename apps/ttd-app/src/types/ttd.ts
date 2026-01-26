// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * Placeholder types for TTD protocol.
 *
 * These will be replaced by Wesley-generated types from ttd-protocol-ts
 * once Phase 1b is complete. For now, they mirror the Rust types in
 * ttd-browser and warp-core.
 */

// ─── Identifiers ─────────────────────────────────────────────────────────────

/** 32-byte hash identifier */
export type Hash = Uint8Array;

/** Worldline identifier (32-byte hash) */
export type WorldlineId = Hash;

/** Cursor handle (from ttd-browser) */
export type CursorId = number;

/** Session handle (from ttd-browser) */
export type SessionId = number;

/** Transaction handle */
export type TxId = bigint;

/** Channel identifier (32-byte hash) */
export type ChannelId = Hash;

// ─── Playback ────────────────────────────────────────────────────────────────

export type PlaybackMode = "Paused" | "Play" | "StepForward" | "StepBack";

export type StepResult = {
  result: "NoOp" | "Advanced" | "Seeked" | "ReachedFrontier";
  tick: bigint;
};

export interface CursorState {
  id: CursorId;
  worldlineId: WorldlineId;
  tick: bigint;
  mode: PlaybackMode;
  maxTick: bigint;
}

// ─── Sessions ────────────────────────────────────────────────────────────────

export interface TruthFrame {
  channel: ChannelId;
  value: Uint8Array;
  valueHash: Hash;
  tick: bigint;
  commitHash: Hash;
}

// ─── Compliance ──────────────────────────────────────────────────────────────

export type ViolationSeverity = "Info" | "Warn" | "Error" | "Fatal";

export interface Violation {
  code: string;
  message: string;
  severity: ViolationSeverity;
}

export interface ComplianceModel {
  isGreen: boolean;
  violations: Violation[];
}

// ─── Obligations ─────────────────────────────────────────────────────────────

export type ObligationStatus = "Pending" | "Satisfied" | "Violated";

export interface Obligation {
  id: string;
  description: string;
  deadlineTick: bigint;
  status: ObligationStatus;
}

export interface ObligationState {
  pending: Obligation[];
  satisfied: Obligation[];
  violated: Obligation[];
}

// ─── Worldlines ──────────────────────────────────────────────────────────────

export interface WorldlineNode {
  id: WorldlineId;
  parentId?: WorldlineId;
  forkTick?: bigint;
  label: string;
  compliance: ComplianceModel;
  children: WorldlineNode[];
}

// ─── Provenance ──────────────────────────────────────────────────────────────

export interface AtomWrite {
  atomId: Hash;
  ruleId: Hash;
  tick: bigint;
  oldValue?: Uint8Array;
  newValue: Uint8Array;
}

export interface ProvenanceChain {
  atomId: Hash;
  writes: AtomWrite[];
}

// ─── State Inspector ─────────────────────────────────────────────────────────

export interface AtomEntry {
  id: Hash;
  typeId: Hash;
  typeName: string;
  value: unknown;
  lastWriteTick: bigint;
  lastWriteRule: string;
}

// ─── Receipts ────────────────────────────────────────────────────────────────

export interface TtdrReceipt {
  version: number;
  worldlineId: WorldlineId;
  tick: bigint;
  commitHash: Hash;
  stateRoot: Hash;
  patchDigest: Hash;
  emissionsDigest: Hash;
}

// ─── Utility ─────────────────────────────────────────────────────────────────

/** Convert a hex string to Uint8Array */
export function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

/** Convert Uint8Array to hex string */
export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/** Truncate a hash for display */
export function truncateHash(hash: Hash, chars = 8): string {
  const hex = bytesToHex(hash);
  return `${hex.slice(0, chars)}…${hex.slice(-chars)}`;
}
