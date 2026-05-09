// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
import { defineConfig } from "vitepress";
import { withMermaid } from "vitepress-plugin-mermaid";

export default withMermaid(
  defineConfig({
    title: "Echo",
    description: "Deterministic WARP runtime docs for Echo",
    cleanUrls: true,
    srcExclude: ["method/**", "design/**"],
    vite: {
      build: {
        chunkSizeWarningLimit: 700,
      },
    },
    themeConfig: {
      search: { provider: "local" },
      nav: [
        { text: "Home", link: "/" },
        { text: "Runtime Model", link: "/architecture/outline" },
        { text: "Theory Map", link: "/theory/THEORY" },
        {
          text: "Specs",
          items: [
            { text: "warp-core", link: "/spec/warp-core" },
            { text: "Scheduler", link: "/spec/scheduler-warp-core" },
            { text: "Tick Patch", link: "/spec/warp-tick-patch" },
            { text: "Merkle Commit", link: "/spec/merkle-commit" },
            { text: "WASM ABI", link: "/spec/SPEC-0009-wasm-abi" },
            { text: "WVP", link: "/spec/warp-view-protocol" },
          ],
        },
      ],
      sidebar: {
        "/": [
          {
            text: "Overview",
            items: [
              { text: "Docs Map", link: "/" },
              { text: "Runtime Model", link: "/architecture/outline" },
              {
                text: "There Is No Graph",
                link: "/architecture/there-is-no-graph",
              },
              {
                text: "Application Contract Hosting",
                link: "/architecture/application-contract-hosting",
              },
              { text: "Theory Map", link: "/theory/THEORY" },
              { text: "Current Bearing", link: "/BEARING" },
            ],
          },
          {
            text: "Kernel Specs",
            items: [
              { text: "warp-core", link: "/spec/warp-core" },
              {
                text: "Attachment Atoms",
                link: "/spec/SPEC-0001-attachment-plane-v0-atoms",
              },
              {
                text: "Descended Attachments",
                link: "/spec/SPEC-0002-descended-attachments-v1",
              },
              {
                text: "DPO Litmus",
                link: "/spec/SPEC-0003-dpo-concurrency-litmus-v0",
              },
              { text: "Scheduler", link: "/spec/scheduler-warp-core" },
              { text: "Tick Patch", link: "/spec/warp-tick-patch" },
              { text: "Merkle Commit", link: "/spec/merkle-commit" },
              {
                text: "Canonical Inbox",
                link: "/spec/canonical-inbox-sequencing",
              },
            ],
          },
          {
            text: "Platform Specs",
            items: [
              {
                text: "Worldlines + Observation",
                link: "/spec/SPEC-0004-worldlines-playback-truthbus",
              },
              {
                text: "Provenance Payload",
                link: "/spec/SPEC-0005-provenance-payload",
              },
              { text: "WASM ABI", link: "/spec/SPEC-0009-wasm-abi" },
              { text: "JS/CBOR Mapping", link: "/spec/js-cbor-mapping" },
              { text: "ABI Golden Vectors", link: "/spec/abi-golden-vectors" },
              { text: "WARP View Protocol", link: "/spec/warp-view-protocol" },
            ],
          },
        ],
      },
    },
  }),
);
