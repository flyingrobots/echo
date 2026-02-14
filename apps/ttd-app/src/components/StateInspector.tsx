// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import type { TtdEngine } from "../hooks/useTtdEngine";
import { useTtdStore } from "../store/ttdStore";
import type { AtomEntry } from "../types/ttd";
import { truncateHash } from "../types/ttd";
import "./StateInspector.css";

interface StateInspectorProps {
  engine: TtdEngine;
}

// Mock atoms for visualization
const mockAtoms: AtomEntry[] = [
  {
    id: new Uint8Array(32).fill(10),
    typeId: new Uint8Array(32).fill(100),
    typeName: "Counter",
    value: { count: 47 },
    lastWriteTick: 45n,
    lastWriteRule: "update_counter",
  },
  {
    id: new Uint8Array(32).fill(11),
    typeId: new Uint8Array(32).fill(101),
    typeName: "Position",
    value: { x: 120.5, y: 340.2 },
    lastWriteTick: 44n,
    lastWriteRule: "physics_step",
  },
  {
    id: new Uint8Array(32).fill(12),
    typeId: new Uint8Array(32).fill(102),
    typeName: "Velocity",
    value: { dx: 2.1, dy: -5.3 },
    lastWriteTick: 44n,
    lastWriteRule: "physics_step",
  },
  {
    id: new Uint8Array(32).fill(13),
    typeId: new Uint8Array(32).fill(103),
    typeName: "Input",
    value: { key: "ArrowRight", pressed: true },
    lastWriteTick: 40n,
    lastWriteRule: "handle_input",
  },
];

export function StateInspector({ engine: _engine }: StateInspectorProps) {
  const selectedAtomId = useTtdStore((s) => s.selectedAtomId);
  const { selectAtom, toggleProvenanceDrawer } = useTtdStore();

  const handleAtomClick = (atom: AtomEntry) => {
    selectAtom(atom.id);
  };

  const handleViewProvenance = () => {
    toggleProvenanceDrawer();
  };

  return (
    <div className="state-inspector panel">
      <div className="panel-header">
        State Inspector
        {selectedAtomId && (
          <button className="btn btn-sm" onClick={handleViewProvenance}>
            View Provenance
          </button>
        )}
      </div>
      <div className="panel-content">
        <div className="atom-list">
          {mockAtoms.map((atom) => {
            const isSelected = selectedAtomId && arraysEqual(atom.id, selectedAtomId);
            return (
              <div
                key={truncateHash(atom.id)}
                className={`atom-row ${isSelected ? "selected" : ""}`}
                onClick={() => handleAtomClick(atom)}
              >
                <div className="atom-header">
                  <span className="atom-type">{atom.typeName}</span>
                  <code className="atom-id">{truncateHash(atom.id, 4)}</code>
                </div>
                <div className="atom-value">
                  <pre>{JSON.stringify(atom.value, null, 2)}</pre>
                </div>
                <div className="atom-meta">
                  <span className="atom-tick">T{atom.lastWriteTick.toString()}</span>
                  <span className="atom-rule">{atom.lastWriteRule}</span>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

function arraysEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}
