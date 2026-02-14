// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import { create } from "zustand";
import type {
  CursorState,
  PlaybackMode,
  WorldlineId,
  WorldlineNode,
  ComplianceModel,
  ObligationStateApp,
  AtomEntry,
} from "../types/ttd";

// ─── Store State ─────────────────────────────────────────────────────────────

interface TtdState {
  // Active cursor
  activeCursor: CursorState | null;

  // Playback state
  isPlaying: boolean;
  playbackSpeed: number; // 1 = normal, 0.5 = half, 2 = double

  // Worldline tree
  worldlineTree: WorldlineNode | null;
  selectedWorldlineId: WorldlineId | null;

  // Compliance & Obligations
  compliance: ComplianceModel;
  obligations: ObligationStateApp;

  // State inspector
  atoms: AtomEntry[];
  selectedAtomId: Uint8Array | null;

  // UI state
  showProvenanceDrawer: boolean;
  show4DView: boolean;
  activePanelId: string;
}

interface TtdActions {
  // Cursor actions
  setActiveCursor: (cursor: CursorState | null) => void;
  updateCursorTick: (tick: bigint) => void;
  setPlaybackMode: (mode: PlaybackMode) => void;

  // Playback controls
  play: () => void;
  pause: () => void;
  stepForward: () => void;
  stepBack: () => void;
  seekTo: (tick: bigint) => void;
  setPlaybackSpeed: (speed: number) => void;

  // Worldline actions
  setWorldlineTree: (tree: WorldlineNode) => void;
  selectWorldline: (id: WorldlineId | null) => void;
  fork: () => void;

  // Compliance
  setCompliance: (compliance: ComplianceModel) => void;
  setObligations: (obligations: ObligationStateApp) => void;

  // State inspector
  setAtoms: (atoms: AtomEntry[]) => void;
  selectAtom: (id: Uint8Array | null) => void;

  // UI actions
  toggleProvenanceDrawer: () => void;
  toggle4DView: () => void;
  setActivePanel: (panelId: string) => void;
}

// ─── Store Implementation ────────────────────────────────────────────────────

export const useTtdStore = create<TtdState & TtdActions>((set, get) => ({
  // Initial state
  activeCursor: null,
  isPlaying: false,
  playbackSpeed: 1,
  worldlineTree: null,
  selectedWorldlineId: null,
  compliance: { isGreen: true, violations: [] },
  obligations: { pending: [], satisfied: [], violated: [] },
  atoms: [],
  selectedAtomId: null,
  showProvenanceDrawer: false,
  show4DView: false,
  activePanelId: "state-inspector",

  // Cursor actions
  setActiveCursor: (cursor) => set({ activeCursor: cursor }),

  updateCursorTick: (tick) =>
    set((state) =>
      state.activeCursor ? { activeCursor: { ...state.activeCursor, tick } } : {}
    ),

  setPlaybackMode: (mode) =>
    set((state) =>
      state.activeCursor
        ? { activeCursor: { ...state.activeCursor, mode } }
        : {}
    ),

  // Playback controls
  play: () => {
    set({ isPlaying: true });
    get().setPlaybackMode("PLAY");
  },

  pause: () => {
    set({ isPlaying: false });
    get().setPlaybackMode("PAUSED");
  },

  stepForward: () => {
    get().setPlaybackMode("STEP_FORWARD");
  },

  stepBack: () => {
    get().setPlaybackMode("STEP_BACK");
  },

  seekTo: (tick) =>
    set((state) =>
      state.activeCursor
        ? { activeCursor: { ...state.activeCursor, tick } }
        : {}
    ),

  setPlaybackSpeed: (speed) => set({ playbackSpeed: speed }),

  // Worldline actions
  setWorldlineTree: (tree) => set({ worldlineTree: tree }),

  selectWorldline: (id) => set({ selectedWorldlineId: id }),

  fork: () => {
    // Will be implemented with engine integration
    console.log("Fork requested");
  },

  // Compliance
  setCompliance: (compliance) => set({ compliance }),
  setObligations: (obligations) => set({ obligations }),

  // State inspector
  setAtoms: (atoms) => set({ atoms }),
  selectAtom: (id) => set({ selectedAtomId: id }),

  // UI actions
  toggleProvenanceDrawer: () =>
    set((state) => ({ showProvenanceDrawer: !state.showProvenanceDrawer })),

  toggle4DView: () => set((state) => ({ show4DView: !state.show4DView })),

  setActivePanel: (panelId) => set({ activePanelId: panelId }),
}));

// ─── Selectors ───────────────────────────────────────────────────────────────

export const selectCurrentTick = (state: TtdState) =>
  state.activeCursor?.tick ?? 0n;

export const selectMaxTick = (state: TtdState) =>
  state.activeCursor?.maxTick ?? 0n;

export const selectIsCompliant = (state: TtdState) => state.compliance.isGreen;

export const selectPendingObligations = (state: TtdState) =>
  state.obligations.pending.length;
