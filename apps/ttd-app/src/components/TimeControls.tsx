// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import type { TtdEngine } from "../hooks/useTtdEngine";
import { useTtdStore, selectCurrentTick, selectMaxTick, selectIsCompliant } from "../store/ttdStore";
import "./TimeControls.css";

interface TimeControlsProps {
  engine: TtdEngine;
}

export function TimeControls({ engine: _engine }: TimeControlsProps) {
  const currentTick = useTtdStore(selectCurrentTick);
  const maxTick = useTtdStore(selectMaxTick);
  const isPlaying = useTtdStore((s) => s.isPlaying);
  const playbackSpeed = useTtdStore((s) => s.playbackSpeed);
  const isCompliant = useTtdStore(selectIsCompliant);

  const { play, pause, stepForward, stepBack, setPlaybackSpeed, fork } = useTtdStore();

  return (
    <div className="time-controls">
      {/* Logo / Title */}
      <div className="time-controls-brand">
        <span className="brand-icon">◈</span>
        <span className="brand-name">Echo TTD</span>
      </div>

      {/* Playback buttons */}
      <div className="time-controls-playback">
        <button
          className="btn btn-icon"
          onClick={stepBack}
          title="Step Back (←)"
        >
          ⏮
        </button>

        {isPlaying ? (
          <button
            className="btn btn-icon btn-primary"
            onClick={pause}
            title="Pause (Space)"
          >
            ⏸
          </button>
        ) : (
          <button
            className="btn btn-icon btn-primary"
            onClick={play}
            title="Play (Space)"
          >
            ▶
          </button>
        )}

        <button
          className="btn btn-icon"
          onClick={stepForward}
          title="Step Forward (→)"
        >
          ⏭
        </button>
      </div>

      {/* Tick display */}
      <div className="time-controls-tick">
        <span className="tick-label">Tick</span>
        <span className="tick-value">{currentTick.toString()}</span>
        <span className="tick-separator">/</span>
        <span className="tick-max">{maxTick.toString()}</span>
      </div>

      {/* Speed control */}
      <div className="time-controls-speed">
        <label>Speed</label>
        <select
          value={playbackSpeed}
          onChange={(e) => setPlaybackSpeed(Number(e.target.value))}
        >
          <option value={0.25}>0.25×</option>
          <option value={0.5}>0.5×</option>
          <option value={1}>1×</option>
          <option value={2}>2×</option>
          <option value={4}>4×</option>
        </select>
      </div>

      {/* Fork button */}
      <button className="btn" onClick={fork} title="Fork worldline">
        ⑂ Fork
      </button>

      {/* Compliance badge */}
      <div className={`badge ${isCompliant ? "badge-green" : "badge-red"}`}>
        {isCompliant ? "✓ Compliant" : "✗ Violations"}
      </div>
    </div>
  );
}
