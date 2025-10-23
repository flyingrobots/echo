/**
 * Echo Core public API skeleton.
 *
 * The goal of this module is to provide a strongly-typed surface that mirrors the
 * architecture specification while we iterate on the deeper implementation.
 */

export type ChronosTick = number;
export type KairosBranchId = string;
export type AionWeight = number;

export interface TimelineFingerprint {
  readonly chronos: ChronosTick;
  readonly kairos: KairosBranchId;
  readonly aion: AionWeight;
}

export interface EngineOptions {
  /**
   * Fixed timestep in milliseconds. Defaults to 33.333 (30Hz) until tuning lands.
   */
  readonly fixedTimeStepMs?: number;
  /**
   * Number of historical frames to retain for branch scrubbing.
   */
  readonly historySize?: number;
}

export interface EngineStats {
  readonly activeBranches: number;
  readonly entropy: number;
  readonly lastTickDurationMs: number;
}

export interface BranchHandle {
  readonly fingerprint: TimelineFingerprint;
}

/**
 * Command envelope that would be queued through Codex's Baby.
 * Placeholder until the event system is implemented.
 */
export interface CommandEnvelope<TPayload = unknown> {
  readonly targetBranch: KairosBranchId;
  readonly chronos: ChronosTick;
  readonly payload: TPayload;
}

type TickCallback = (context: TickContext) => void;

export interface TickContext {
  readonly fingerprint: TimelineFingerprint;
  readonly deltaTimeMs: number;
}

/**
 * Minimal placeholder for the engine runtime. The implementation will grow to include the
 * deterministic scheduler, archetype storage, Codex's Baby, and branch tree orchestration.
 */
const now = (): number => {
  if (typeof performance !== "undefined" && typeof performance.now === "function") {
    return performance.now();
  }
  return Date.now();
};

const createBranchId = (): string => {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return Math.random().toString(36).slice(2);
};

export class EchoEngine {
  private readonly options: Required<EngineOptions>;
  private readonly tickHandlers: Set<TickCallback> = new Set();
  private statsInternal: EngineStats = {
    activeBranches: 1,
    entropy: 0,
    lastTickDurationMs: 0
  };

  public constructor(options: EngineOptions = {}) {
    this.options = {
      fixedTimeStepMs: options.fixedTimeStepMs ?? 1000 / 60,
      historySize: options.historySize ?? 256
    };
  }

  public get stats(): EngineStats {
    return this.statsInternal;
  }

  public registerTickHandler(handler: TickCallback): () => void {
    this.tickHandlers.add(handler);
    return () => this.tickHandlers.delete(handler);
  }

  /**
   * Placeholder tick loop. Emits a static context so downstream systems can begin integrating.
   */
  public tick(): void {
    const start = now();
    const context: TickContext = {
      fingerprint: {
        chronos: 0,
        kairos: "prime",
        aion: 1
      },
      deltaTimeMs: this.options.fixedTimeStepMs
    };
    for (const handler of this.tickHandlers) {
      handler(context);
    }
    const end = now();
    this.statsInternal = {
      ...this.statsInternal,
      lastTickDurationMs: end - start
    };
  }

  /**
     * Spawn a speculative branch. Currently returns a stub so call sites can be wired.
     */
  public forkBranch(): BranchHandle {
    return {
      fingerprint: {
        chronos: 0,
        kairos: createBranchId(),
        aion: 0.5
      }
    };
  }

  public queueCommand(_envelope: CommandEnvelope): void {
    // TODO: integrate with Codex's Baby.
  }
}
