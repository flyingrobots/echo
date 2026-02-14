// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import { useTtdEngine } from "./hooks/useTtdEngine";
import { Layout } from "./views/Layout";

export function App() {
  const { engine, state, error } = useTtdEngine();

  if (state === "loading") {
    return (
      <div className="loading-screen">
        <div className="loading-content">
          <div className="loading-spinner" />
          <p>Loading TTD Engine...</p>
        </div>
        <style>{`
          .loading-screen {
            width: 100vw;
            height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            background: var(--bg-primary);
          }
          .loading-content {
            text-align: center;
          }
          .loading-spinner {
            width: 40px;
            height: 40px;
            border: 3px solid var(--border-color);
            border-top-color: var(--accent-blue);
            border-radius: 50%;
            animation: spin 1s linear infinite;
            margin: 0 auto 16px;
          }
          @keyframes spin {
            to { transform: rotate(360deg); }
          }
        `}</style>
      </div>
    );
  }

  if (state === "error") {
    return (
      <div className="error-screen">
        <div className="error-content">
          <h1>Failed to Load</h1>
          <p>{error}</p>
          <button className="btn btn-primary" onClick={() => window.location.reload()}>
            Retry
          </button>
        </div>
        <style>{`
          .error-screen {
            width: 100vw;
            height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            background: var(--bg-primary);
          }
          .error-content {
            text-align: center;
          }
          .error-content h1 {
            color: var(--accent-red);
            margin-bottom: 8px;
          }
          .error-content p {
            color: var(--text-secondary);
            margin-bottom: 16px;
          }
        `}</style>
      </div>
    );
  }

  return <Layout engine={engine!} />;
}
