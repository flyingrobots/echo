// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Echo',
  description: 'Real-Time, Deterministic, Recursive Meta-Graph Simulation Engine',
  cleanUrls: true,
  themeConfig: {
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Start Here', link: '/guide/start-here' },
      { text: 'Docs Map', link: '/docs-index' },
      {
        text: 'Guides',
        items: [
          { text: 'WARP Primer', link: '/guide/warp-primer' },
          { text: 'WVP Demo', link: '/guide/wvp-demo' },
          { text: 'Collision Tour', link: '/guide/collision-tour' }
        ]
      },
      {
        text: 'Specs',
        items: [
          { text: 'warp-core', link: '/spec-warp-core' },
          { text: 'Tick Patch', link: '/spec-warp-tick-patch' },
          { text: 'Serialization', link: '/spec-serialization-protocol' },
          { text: 'Branch Tree', link: '/spec-branch-tree' },
          { text: 'WVP', link: '/spec-warp-view-protocol' }
        ]
      },
      {
        text: 'Log',
        items: [
          { text: 'Execution Plan', link: '/execution-plan' },
          { text: 'Decision Log', link: '/decision-log' }
        ]
      }
    ],
    sidebar: {
      '/': [
        {
          text: 'Start Here',
          items: [
            { text: 'Start Here', link: '/guide/start-here' },
            { text: 'WARP Primer', link: '/guide/warp-primer' },
            { text: 'Docs Map', link: '/docs-index' },
            { text: 'Architecture Outline', link: '/architecture-outline' }
          ]
        },
        {
          text: 'WARP',
          items: [
            { text: 'warp-core Spec', link: '/spec-warp-core' },
            { text: 'Tick Patch Spec', link: '/spec-warp-tick-patch' },
            { text: 'WVP Spec', link: '/spec-warp-view-protocol' },
            { text: 'Serialization Spec', link: '/spec-serialization-protocol' },
            { text: 'Branch Tree Spec', link: '/spec-branch-tree' }
          ]
        },
        {
          text: 'Subsystem Hubs',
          items: [{ text: 'Scheduler', link: '/scheduler' }]
        },
        {
          text: 'Project Log',
          items: [
            { text: 'Execution Plan', link: '/execution-plan' },
            { text: 'Decision Log', link: '/decision-log' }
          ]
        }
      ],
      '/guide/': [
        {
          text: 'Guide',
          items: [
            { text: 'Start Here', link: '/guide/start-here' },
            { text: 'WARP Primer', link: '/guide/warp-primer' },
            { text: 'WVP Demo', link: '/guide/wvp-demo' },
            { text: 'Collision Tour', link: '/guide/collision-tour' }
          ]
        }
      ]
    }
  }
})
