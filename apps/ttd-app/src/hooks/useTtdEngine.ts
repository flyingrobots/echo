// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import { useEffect, useState } from "react";

/**
 * TtdEngine wrapper interface.
 *
 * This will eventually import from ttd-browser WASM package.
 * For now, it's a placeholder that simulates the API.
 */
export interface TtdEngine {
  // Worldline management
  registerWorldline(worldlineId: Uint8Array, warpId: Uint8Array): void;

  // Cursor management
  createCursor(worldlineId: Uint8Array): number;
  seekTo(cursorId: number, tick: bigint): boolean;
  step(cursorId: number): Uint8Array; // CBOR-encoded StepResult
  getTick(cursorId: number): bigint;
  setMode(cursorId: number, mode: string): void;
  setSeek(cursorId: number, target: bigint, thenPlay: boolean): void;
  updateFrontier(cursorId: number, maxTick: bigint): void;
  dropCursor(cursorId: number): void;

  // Provenance queries
  getStateRoot(cursorId: number): Uint8Array;
  getCommitHash(cursorId: number): Uint8Array;
  getEmissionsDigest(cursorId: number): Uint8Array;
  getHistoryLength(worldlineId: Uint8Array): bigint;

  // Session management
  createSession(): number;
  setSessionCursor(sessionId: number, cursorId: number): void;
  subscribe(sessionId: number, channel: Uint8Array): void;
  unsubscribe(sessionId: number, channel: Uint8Array): void;
  publishTruth(sessionId: number, cursorId: number): void;
  drainFrames(sessionId: number): Uint8Array; // CBOR-encoded TruthFrame[]
  dropSession(sessionId: number): void;

  // Transactions
  begin(cursorId: number): bigint;
  commit(txId: bigint): Uint8Array; // TTDR receipt

  // Fork
  snapshot(cursorId: number): Uint8Array;
  forkFromSnapshot(snapshot: Uint8Array, newWorldlineId: Uint8Array): number;

  // Compliance (stubs)
  getCompliance(): Uint8Array;
  getObligations(): Uint8Array;
}

export type EngineState = "loading" | "ready" | "error";

/**
 * Hook to initialize and manage the TTD WASM engine.
 *
 * Usage:
 * ```tsx
 * const { engine, state, error } = useTtdEngine();
 *
 * if (state === 'loading') return <Loading />;
 * if (state === 'error') return <Error message={error} />;
 *
 * // Use engine...
 * ```
 */
export function useTtdEngine(): {
  engine: TtdEngine | null;
  state: EngineState;
  error: string | null;
} {
  const [engine, setEngine] = useState<TtdEngine | null>(null);
  const [state, setState] = useState<EngineState>("loading");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function initEngine() {
      try {
        // TODO: Replace with actual WASM import once ttd-browser is built
        // const { default: init, TtdEngine } = await import('ttd-browser');
        // await init();
        // const engine = new TtdEngine();

        // For now, create a mock engine for UI development
        const mockEngine = createMockEngine();

        if (!cancelled) {
          setEngine(mockEngine);
          setState("ready");
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "Failed to load WASM");
          setState("error");
        }
      }
    }

    initEngine();
    return () => {
      cancelled = true;
    };
  }, []);

  return { engine, state, error };
}

/**
 * Create a mock engine for UI development.
 * This simulates the ttd-browser API without actual WASM.
 */
function createMockEngine(): TtdEngine {
  let nextCursorId = 1;
  let nextSessionId = 1;
  let nextTxId = 1n;
  const cursors = new Map<number, { tick: bigint; worldlineId: Uint8Array }>();

  return {
    registerWorldline(_worldlineId: Uint8Array, _warpId: Uint8Array) {
      console.log("[mock] registerWorldline");
    },

    createCursor(worldlineId: Uint8Array): number {
      const id = nextCursorId++;
      cursors.set(id, { tick: 0n, worldlineId });
      console.log("[mock] createCursor:", id);
      return id;
    },

    seekTo(cursorId: number, tick: bigint): boolean {
      const cursor = cursors.get(cursorId);
      if (cursor) cursor.tick = tick;
      console.log("[mock] seekTo:", cursorId, tick);
      return true;
    },

    step(cursorId: number): Uint8Array {
      const cursor = cursors.get(cursorId);
      if (cursor) cursor.tick++;
      // Return mock CBOR-encoded StepResult
      return new Uint8Array([0xa2, 0x66, 0x72, 0x65, 0x73, 0x75, 0x6c, 0x74]);
    },

    getTick(cursorId: number): bigint {
      return cursors.get(cursorId)?.tick ?? 0n;
    },

    setMode(cursorId: number, mode: string) {
      console.log("[mock] setMode:", cursorId, mode);
    },

    setSeek(cursorId: number, target: bigint, thenPlay: boolean) {
      console.log("[mock] setSeek:", cursorId, target, thenPlay);
    },

    updateFrontier(cursorId: number, maxTick: bigint) {
      console.log("[mock] updateFrontier:", cursorId, maxTick);
    },

    dropCursor(cursorId: number) {
      cursors.delete(cursorId);
      console.log("[mock] dropCursor:", cursorId);
    },

    getStateRoot(_cursorId: number): Uint8Array {
      return new Uint8Array(32);
    },

    getCommitHash(_cursorId: number): Uint8Array {
      return new Uint8Array(32);
    },

    getEmissionsDigest(_cursorId: number): Uint8Array {
      return new Uint8Array(32);
    },

    getHistoryLength(_worldlineId: Uint8Array): bigint {
      return 100n; // Mock 100 ticks of history
    },

    createSession(): number {
      return nextSessionId++;
    },

    setSessionCursor(sessionId: number, cursorId: number) {
      console.log("[mock] setSessionCursor:", sessionId, cursorId);
    },

    subscribe(sessionId: number, _channel: Uint8Array) {
      console.log("[mock] subscribe:", sessionId);
    },

    unsubscribe(sessionId: number, _channel: Uint8Array) {
      console.log("[mock] unsubscribe:", sessionId);
    },

    publishTruth(sessionId: number, cursorId: number) {
      console.log("[mock] publishTruth:", sessionId, cursorId);
    },

    drainFrames(_sessionId: number): Uint8Array {
      return new Uint8Array([0x80]); // Empty CBOR array
    },

    dropSession(sessionId: number) {
      console.log("[mock] dropSession:", sessionId);
    },

    begin(cursorId: number): bigint {
      console.log("[mock] begin:", cursorId);
      return nextTxId++;
    },

    commit(txId: bigint): Uint8Array {
      console.log("[mock] commit:", txId);
      return new Uint8Array(256); // Mock TTDR receipt
    },

    snapshot(cursorId: number): Uint8Array {
      console.log("[mock] snapshot:", cursorId);
      return new Uint8Array(64);
    },

    forkFromSnapshot(_snapshot: Uint8Array, _newWorldlineId: Uint8Array): number {
      return nextCursorId++;
    },

    getCompliance(): Uint8Array {
      // Mock CBOR: { isGreen: true, violations: [] }
      return new Uint8Array([0xa2, 0x67, 0x69, 0x73, 0x47, 0x72, 0x65, 0x65, 0x6e, 0xf5]);
    },

    getObligations(): Uint8Array {
      // Mock CBOR: { pending: [], satisfied: [], violated: [] }
      return new Uint8Array([0xa3]);
    },
  };
}
