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
      { text: 'Docs Index', link: '/docs-index' },
      { text: 'WARP Primer', link: '/guide/warp-primer' },
      { text: 'Collision Tour', link: '/guide/collision-tour' }
    ],
    sidebar: {
      '/': [
        {
          text: 'Start Here',
          items: [
            { text: 'Docs Index', link: '/docs-index' },
            { text: 'Architecture Outline', link: '/architecture-outline' },
            { text: 'Execution Plan', link: '/execution-plan' },
            { text: 'Decision Log', link: '/decision-log' }
          ]
        },
        {
          text: 'WARP',
          items: [
            { text: 'WARP Primer', link: '/guide/warp-primer' },
            { text: 'warp-core Spec', link: '/spec-warp-core' },
            { text: 'Scheduler (Doc Map)', link: '/scheduler' }
          ]
        }
      ],
      '/guide/': [
        {
          text: 'Guide',
          items: [
            { text: 'WARP Primer', link: '/guide/warp-primer' },
            { text: 'Collision Tour', link: '/guide/collision-tour' }
          ]
        }
      ]
    }
  }
})
