// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import type { TtdEngine } from "../hooks/useTtdEngine";
import { useTtdStore } from "../store/ttdStore";
import type { WorldlineNode } from "../types/ttd";
import { truncateHash } from "../types/ttd";
import { ViolationSeverity } from "@echo/ttd-protocol-ts";
import "./WorldlineTree.css";

interface WorldlineTreeProps {
  engine: TtdEngine;
}

// Mock data for visualization
const mockTree: WorldlineNode = {
  id: new Uint8Array(32).fill(1),
  label: "Main Timeline",
  compliance: { isGreen: true, violations: [] },
  children: [
    {
      id: new Uint8Array(32).fill(2),
      parentId: new Uint8Array(32).fill(1),
      forkTick: 25n,
      label: "Fork A (1 worker)",
      compliance: { isGreen: true, violations: [] },
      children: [],
    },
    {
      id: new Uint8Array(32).fill(3),
      parentId: new Uint8Array(32).fill(1),
      forkTick: 25n,
      label: "Fork B (16 workers)",
      compliance: { isGreen: true, violations: [] },
      children: [
        {
          id: new Uint8Array(32).fill(4),
          parentId: new Uint8Array(32).fill(3),
          forkTick: 50n,
          label: "Fork B.1 (nudged)",
          compliance: {
            isGreen: false,
            violations: [{ code: "V001", message: "Deadline missed", severity: ViolationSeverity.ERROR, channelId: undefined, tick: undefined, emissionCount: undefined }],
          },
          children: [],
        },
      ],
    },
  ],
};

export function WorldlineTree({ engine: _engine }: WorldlineTreeProps) {
  const selectedWorldlineId = useTtdStore((s) => s.selectedWorldlineId);
  const { selectWorldline } = useTtdStore();

  return (
    <div className="worldline-tree panel">
      <div className="panel-header">Worldlines</div>
      <div className="panel-content">
        <TreeNode
          node={mockTree}
          depth={0}
          selectedId={selectedWorldlineId}
          onSelect={selectWorldline}
        />
      </div>
    </div>
  );
}

interface TreeNodeProps {
  node: WorldlineNode;
  depth: number;
  selectedId: Uint8Array | null;
  onSelect: (id: Uint8Array) => void;
}

function TreeNode({ node, depth, selectedId, onSelect }: TreeNodeProps) {
  const isSelected =
    selectedId && arraysEqual(node.id, selectedId);
  const hasViolations = !node.compliance.isGreen;

  return (
    <div className="tree-node">
      <div
        className={`tree-node-row ${isSelected ? "selected" : ""}`}
        style={{ paddingLeft: `${depth * 16 + 8}px` }}
        onClick={() => onSelect(node.id)}
      >
        {node.children.length > 0 && (
          <span className="tree-expand">▾</span>
        )}
        <span className="tree-icon">◈</span>
        <span className="tree-label">{node.label}</span>
        {node.forkTick !== undefined && (
          <span className="tree-fork-tick">@{node.forkTick.toString()}</span>
        )}
        <span
          className={`tree-badge ${hasViolations ? "badge-red" : "badge-green"}`}
        >
          {hasViolations ? "!" : "✓"}
        </span>
      </div>

      {node.children.length > 0 && (
        <div className="tree-children">
          {node.children.map((child, i) => (
            <TreeNode
              key={i}
              node={child}
              depth={depth + 1}
              selectedId={selectedId}
              onSelect={onSelect}
            />
          ))}
        </div>
      )}

      {isSelected && (
        <div className="tree-details" style={{ marginLeft: `${depth * 16 + 24}px` }}>
          <div className="detail-row">
            <span className="detail-label">ID</span>
            <code className="detail-value">{truncateHash(node.id)}</code>
          </div>
          {node.compliance.violations.length > 0 && (
            <div className="detail-violations">
              {node.compliance.violations.map((v, i) => (
                <div key={i} className="violation-item">
                  <span className="violation-code">{v.code}</span>
                  <span className="violation-msg">{v.message}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
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
