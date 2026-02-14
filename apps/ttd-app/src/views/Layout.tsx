// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import type { TtdEngine } from "../hooks/useTtdEngine";
import { TimeControls } from "../components/TimeControls";
import { Timeline } from "../components/Timeline";
import { WorldlineTree } from "../components/WorldlineTree";
import { StateInspector } from "../components/StateInspector";
import { ProvenanceDrawer } from "../components/ProvenanceDrawer";
import { useTtdStore } from "../store/ttdStore";
import "./Layout.css";

interface LayoutProps {
  engine: TtdEngine;
}

export function Layout({ engine }: LayoutProps) {
  const showProvenanceDrawer = useTtdStore((s) => s.showProvenanceDrawer);

  return (
    <div className="ttd-layout">
      {/* Top: Time Controls */}
      <header className="ttd-header">
        <TimeControls engine={engine} />
      </header>

      {/* Main content area */}
      <div className="ttd-main">
        {/* Left sidebar: Worldline Tree */}
        <aside className="ttd-sidebar-left">
          <WorldlineTree engine={engine} />
        </aside>

        {/* Center: 3D View placeholder */}
        <main className="ttd-center">
          <div className="panel ttd-3d-view">
            <div className="panel-header">4D Provenance View</div>
            <div className="panel-content ttd-3d-canvas">
              <div className="ttd-3d-placeholder">
                <p>Three.js visualization will render here</p>
                <p className="hint">
                  Connect <code>@echo/renderer-three</code> to enable
                </p>
              </div>
            </div>
          </div>
        </main>

        {/* Right sidebar: State Inspector */}
        <aside className="ttd-sidebar-right">
          <StateInspector engine={engine} />
        </aside>
      </div>

      {/* Bottom: Timeline */}
      <footer className="ttd-footer">
        <Timeline engine={engine} />
      </footer>

      {/* Provenance Drawer (slide-out) */}
      {showProvenanceDrawer && <ProvenanceDrawer engine={engine} />}
    </div>
  );
}
