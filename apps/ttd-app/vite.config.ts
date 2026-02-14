// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import wasm from "vite-plugin-wasm";

export default defineConfig({
  plugins: [react(), wasm()],
  build: {
    target: "esnext",
  },
  optimizeDeps: {
    exclude: ["ttd-browser"],
  },
  server: {
    fs: {
      // Allow serving WASM from crates directory
      allow: [".", "../../crates/ttd-browser/pkg"],
    },
  },
});
