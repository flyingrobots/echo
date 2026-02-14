// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import type { TtdEngine } from "../hooks/useTtdEngine";
import { useTtdStore } from "../store/ttdStore";
import type { AtomWrite } from "../types/ttd";
import { truncateHash } from "../types/ttd";
import "./ProvenanceDrawer.css";

interface ProvenanceDrawerProps {
  engine: TtdEngine;
}

// Mock provenance data
const mockWrites: AtomWrite[] = [
  {
    atomId: new Uint8Array(32).fill(10),
    ruleId: new Uint8Array(32).fill(200),
    tick: 45n,
    oldValue: new TextEncoder().encode(JSON.stringify({ count: 46 })),
    newValue: new TextEncoder().encode(JSON.stringify({ count: 47 })),
  },
  {
    atomId: new Uint8Array(32).fill(10),
    ruleId: new Uint8Array(32).fill(200),
    tick: 40n,
    oldValue: new TextEncoder().encode(JSON.stringify({ count: 45 })),
    newValue: new TextEncoder().encode(JSON.stringify({ count: 46 })),
  },
  {
    atomId: new Uint8Array(32).fill(10),
    ruleId: new Uint8Array(32).fill(200),
    tick: 35n,
    oldValue: new TextEncoder().encode(JSON.stringify({ count: 44 })),
    newValue: new TextEncoder().encode(JSON.stringify({ count: 45 })),
  },
  {
    atomId: new Uint8Array(32).fill(10),
    ruleId: new Uint8Array(32).fill(201),
    tick: 0n,
    oldValue: undefined,
    newValue: new TextEncoder().encode(JSON.stringify({ count: 0 })),
  },
];

export function ProvenanceDrawer({ engine: _engine }: ProvenanceDrawerProps) {
  const selectedAtomId = useTtdStore((s) => s.selectedAtomId);
  const { toggleProvenanceDrawer, toggle4DView } = useTtdStore();

  const decodeValue = (bytes?: Uint8Array): string => {
    if (!bytes) return "(created)";
    try {
      return new TextDecoder().decode(bytes);
    } catch {
      return truncateHash(bytes);
    }
  };

  return (
    <div className="provenance-drawer">
      <div className="drawer-header">
        <h3>Provenance: {selectedAtomId ? truncateHash(selectedAtomId) : "—"}</h3>
        <div className="drawer-actions">
          <button className="btn" onClick={toggle4DView}>
            View in 4D
          </button>
          <button className="btn" onClick={toggleProvenanceDrawer}>
            ✕
          </button>
        </div>
      </div>

      <div className="drawer-content">
        <div className="provenance-timeline">
          {mockWrites.map((write, i) => (
            <div key={i} className="provenance-entry">
              <div className="entry-tick">
                <span className="tick-marker" />
                <span className="tick-value">T{write.tick.toString()}</span>
              </div>

              <div className="entry-content">
                <div className="entry-rule">
                  <span className="label">Rule</span>
                  <code>{truncateHash(write.ruleId)}</code>
                </div>

                <div className="entry-diff">
                  {write.oldValue && (
                    <div className="diff-old">
                      <span className="diff-marker">−</span>
                      <pre>{decodeValue(write.oldValue)}</pre>
                    </div>
                  )}
                  <div className="diff-new">
                    <span className="diff-marker">+</span>
                    <pre>{decodeValue(write.newValue)}</pre>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>

        <div className="provenance-summary">
          <h4>Summary</h4>
          <div className="summary-stats">
            <div className="stat">
              <span className="stat-value">{mockWrites.length}</span>
              <span className="stat-label">Total Writes</span>
            </div>
            <div className="stat">
              <span className="stat-value">1</span>
              <span className="stat-label">Rules Involved</span>
            </div>
            <div className="stat">
              <span className="stat-value">T0</span>
              <span className="stat-label">Created At</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
