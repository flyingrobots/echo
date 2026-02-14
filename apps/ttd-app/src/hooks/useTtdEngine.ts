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
  register_worldline(worldline_id: Uint8Array, warp_id: Uint8Array): void;

  // Cursor management
  create_cursor(worldline_id: Uint8Array): number;
  seek_to(cursor_id: number, tick: bigint): boolean;
  step(cursor_id: number): Uint8Array; // CBOR-encoded StepResult
  get_tick(cursor_id: number): bigint;
  set_mode(cursor_id: number, mode: string): void;
  set_seek(cursor_id: number, target: bigint, then_play: boolean): void;
  update_frontier(cursor_id: number, max_tick: bigint): void;
  drop_cursor(cursor_id: number): void;

  // Provenance queries
  get_state_root(cursor_id: number): Uint8Array;
  get_commit_hash(cursor_id: number): Uint8Array;
  get_emissions_digest(cursor_id: number): Uint8Array;
  get_history_length(worldline_id: Uint8Array): bigint;

  // Session management
  create_session(): number;
  set_session_cursor(session_id: number, cursor_id: number): void;
  subscribe(session_id: number, channel: Uint8Array): void;
  unsubscribe(session_id: number, channel: Uint8Array): void;
  publish_truth(session_id: number, cursor_id: number): void;
  drain_frames(session_id: number): Uint8Array; // CBOR-encoded TruthFrame[]
  drop_session(session_id: number): void;

  // Transactions
  begin(cursor_id: number): bigint;
  commit(tx_id: bigint): Uint8Array; // TTDR receipt

  // Fork
  snapshot(cursor_id: number): Uint8Array;
  fork_from_snapshot(snapshot: Uint8Array, new_worldline_id: Uint8Array): number;

  // Compliance (stubs)
  get_compliance(): Uint8Array;
  get_obligations(): Uint8Array;
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
        let engineInstance: TtdEngine;

        try {
          // Attempt actual WASM import
          // @ts-ignore
          const wasm = await import("ttd-browser");
          // wasm-bindgen init might be different depending on build
          if (wasm.default && typeof wasm.default === "function") {
            await wasm.default();
          }
          engineInstance = new wasm.TtdEngine() as TtdEngine;
          console.log("[ttd] WASM engine initialized");
        } catch (wasmErr) {
          console.warn("[ttd] Failed to load WASM engine, falling back to mock:", wasmErr);
          engineInstance = createMockEngine();
        }

        if (!cancelled) {
          setEngine(engineInstance);
          setState("ready");
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "Failed to load engine");
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
    register_worldline(_worldlineId: Uint8Array, _warpId: Uint8Array) {
      console.log("[mock] registerWorldline");
    },

    create_cursor(worldlineId: Uint8Array): number {
      const id = nextCursorId++;
      cursors.set(id, { tick: 0n, worldlineId });
      console.log("[mock] createCursor:", id);
      return id;
    },

    seek_to(cursorId: number, tick: bigint): boolean {
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

    get_tick(cursorId: number): bigint {
      return cursors.get(cursorId)?.tick ?? 0n;
    },

    set_mode(cursorId: number, mode: string) {
      console.log("[mock] setMode:", cursorId, mode);
    },

    set_seek(cursorId: number, target: bigint, thenPlay: boolean) {
      console.log("[mock] setSeek:", cursorId, target, thenPlay);
    },

    update_frontier(cursor_id: number, maxTick: bigint) {
      console.log("[mock] updateFrontier:", cursor_id, maxTick);
    },

    drop_cursor(cursorId: number) {
      cursors.delete(cursorId);
      console.log("[mock] dropCursor:", cursorId);
    },

    get_state_root(_cursorId: number): Uint8Array {
      return new Uint8Array(32);
    },

    get_commit_hash(_cursorId: number): Uint8Array {
      return new Uint8Array(32);
    },

    get_emissions_digest(_cursorId: number): Uint8Array {
      return new Uint8Array(32);
    },

    get_history_length(_worldlineId: Uint8Array): bigint {
      return 100n; // Mock 100 ticks of history
    },

    create_session(): number {
      return nextSessionId++;
    },

    set_session_cursor(sessionId: number, cursorId: number) {
      console.log("[mock] setSessionCursor:", sessionId, cursorId);
    },

    subscribe(sessionId: number, _channel: Uint8Array) {
      console.log("[mock] subscribe:", sessionId);
    },

    unsubscribe(sessionId: number, _channel: Uint8Array) {
      console.log("[mock] unsubscribe:", sessionId);
    },

    publish_truth(sessionId: number, cursorId: number) {
      console.log("[mock] publishTruth:", sessionId, cursorId);
    },

    drain_frames(_sessionId: number): Uint8Array {
      return new Uint8Array([0x80]); // Empty CBOR array
    },

    drop_session(sessionId: number) {
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

    fork_from_snapshot(_snapshot: Uint8Array, _newWorldlineId: Uint8Array): number {
      return nextCursorId++;
    },

    get_compliance(): Uint8Array {
      // Mock CBOR: { isGreen: true, violations: [] }
      return new Uint8Array([0xa2, 0x67, 0x69, 0x73, 0x47, 0x72, 0x65, 0x65, 0x6e, 0xf5]);
    },

    get_obligations(): Uint8Array {
      // Mock CBOR: { pending: [], satisfied: [], violated: [] }
      return new Uint8Array([0xa3]);
    },
  };
}
