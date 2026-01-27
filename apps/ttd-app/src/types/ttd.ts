// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

/**
 * TTD Protocol Types & App-Specific Utilities
 *
 * Re-exports types from @echo/ttd-protocol-ts (Wesley-generated).
 * Keeps utility functions and app-specific types that are not in the protocol.
 */

// ─── Import Protocol Types ───────────────────────────────────────────────────

import type {
  Hash,
  CursorRole,
  SeekResult,
  ComplianceStatus,
  ViolationSeverity,
  StepResultKind,
  CursorMoved,
  SeekCompleted,
  SeekFailed,
  ViolationDetected,
  ComplianceUpdate,
  SessionStarted,
  SessionEnded,
  CursorCreated,
  CursorDestroyed,
  Violation,
  TruthFrame,
  ObligationState,
  StepResult,
  Snapshot,
  ComplianceModel,
  Obligation,
  ObligationReport,
  TtdSystem,
} from "@echo/ttd-protocol-ts";

// ─── Re-export Protocol Types ────────────────────────────────────────────────

export type {
  Hash,
  CursorRole,
  SeekResult,
  ComplianceStatus,
  ViolationSeverity,
  StepResultKind,
  CursorMoved,
  SeekCompleted,
  SeekFailed,
  ViolationDetected,
  ComplianceUpdate,
  SessionStarted,
  SessionEnded,
  CursorCreated,
  CursorDestroyed,
  Violation,
  TruthFrame,
  ObligationState,
  StepResult,
  Snapshot,
  ComplianceModel,
  Obligation,
  ObligationReport,
  TtdSystem,
};

// ─── App-Specific Type Aliases & Helpers ─────────────────────────────────────

/** Worldline identifier (32-byte hash) */
export type WorldlineId = Hash;

/** Channel identifier (32-byte hash) */
export type ChannelId = Hash;

/**
 * App-specific PlaybackMode type
 * Maps to the protocol's PlaybackMode enum with string literal values
 */
export type PlaybackMode = "PAUSED" | "PLAY" | "STEP_FORWARD" | "STEP_BACK" | "SEEK";

// ─── App-Specific Cursor State (extends protocol) ─────────────────────────────

/** Extended cursor state with app-specific fields */
export interface CursorState {
  id: number;
  worldlineId: WorldlineId;
  tick: bigint;
  mode: PlaybackMode;
  maxTick: bigint;
}

// ─── App-Specific Obligation State (extends protocol) ──────────────────────────

/** Extended obligation state with typed status */
export type ObligationStatus = "Pending" | "Satisfied" | "Violated";

export interface ObligationStateApp {
  pending: Array<{ id: string; description: string; deadlineTick: bigint }>;
  satisfied: Array<{ id: string; description: string; deadlineTick: bigint }>;
  violated: Array<{ id: string; description: string; deadlineTick: bigint }>;
}

// ─── Worldlines (app-specific) ────────────────────────────────────────────────

export interface WorldlineNode {
  id: WorldlineId;
  parentId?: WorldlineId;
  forkTick?: bigint;
  label: string;
  compliance: ComplianceModel;
  children: WorldlineNode[] ;
}

// ─── Provenance (app-specific) ────────────────────────────────────────────────

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

// ─── State Inspector (app-specific) ───────────────────────────────────────────

export interface AtomEntry {
  id: Hash;
  typeId: Hash;
  typeName: string;
  value: unknown;
  lastWriteTick: bigint;
  lastWriteRule: string;
}

// ─── Receipts (app-specific) ─────────────────────────────────────────────────

export interface TtdrReceipt {
  version: number;
  worldlineId: WorldlineId;
  tick: bigint;
  commitHash: Hash;
  stateRoot: Hash;
  patchDigest: Hash;
  emissionsDigest: Hash;
}

// ─── Utility Functions ────────────────────────────────────────────────────────

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
