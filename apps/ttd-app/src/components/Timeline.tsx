// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import { useCallback } from "react";
import type { TtdEngine } from "../hooks/useTtdEngine";
import { useTtdStore, selectCurrentTick, selectMaxTick } from "../store/ttdStore";
import "./Timeline.css";

interface TimelineProps {
  engine: TtdEngine;
}

export function Timeline({ engine: _engine }: TimelineProps) {
  const currentTick = useTtdStore(selectCurrentTick);
  const maxTick = useTtdStore(selectMaxTick);
  const { seekTo } = useTtdStore();

  // Mock markers for visualization
  const markers = [
    { tick: 10n, type: "intent", label: "ClickIntent" },
    { tick: 25n, type: "fork", label: "Fork A" },
    { tick: 45n, type: "rule", label: "update_counter" },
    { tick: 72n, type: "violation", label: "Deadline missed" },
  ];

  const handleScrub = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      const rect = e.currentTarget.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const ratio = x / rect.width;
      const tick = BigInt(Math.round(Number(maxTick) * ratio));
      seekTo(tick);
    },
    [maxTick, seekTo]
  );

  const progress =
    maxTick > 0n ? (Number(currentTick) / Number(maxTick)) * 100 : 0;

  return (
    <div className="timeline">
      <div className="timeline-track" onClick={handleScrub}>
        {/* Progress bar */}
        <div className="timeline-progress" style={{ width: `${progress}%` }} />

        {/* Playhead */}
        <div
          className="timeline-playhead"
          style={{ left: `${progress}%` }}
        />

        {/* Markers */}
        {markers.map((marker, i) => {
          const pos =
            maxTick > 0n
              ? (Number(marker.tick) / Number(maxTick)) * 100
              : 0;
          return (
            <div
              key={i}
              className={`timeline-marker timeline-marker-${marker.type}`}
              style={{ left: `${pos}%` }}
              title={`${marker.label} @ T${marker.tick}`}
            />
          );
        })}
      </div>

      {/* Tick labels */}
      <div className="timeline-labels">
        <span>0</span>
        <span>{(Number(maxTick) / 4).toFixed(0)}</span>
        <span>{(Number(maxTick) / 2).toFixed(0)}</span>
        <span>{((Number(maxTick) * 3) / 4).toFixed(0)}</span>
        <span>{maxTick.toString()}</span>
      </div>

      {/* Legend */}
      <div className="timeline-legend">
        <span className="legend-item legend-intent">◆ Intent</span>
        <span className="legend-item legend-fork">◆ Fork</span>
        <span className="legend-item legend-rule">◆ Rule Fire</span>
        <span className="legend-item legend-violation">◆ Violation</span>
      </div>
    </div>
  );
}
