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
      { text: 'Collision Tour', link: '/guide/collision-tour' },
    ],
    sidebar: {
      '/guide/': [
        {
          text: 'Guide',
          items: [
            { text: 'Collision Tour', link: '/guide/collision-tour' },
          ]
        }
      ]
    }
  }
})
