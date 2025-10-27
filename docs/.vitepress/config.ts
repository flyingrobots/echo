import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Echo',
  description: 'Deterministic, multiverse-aware ECS',
  cleanUrls: true,
  ignoreDeadLinks: [
    // Collison tour HTML is added in a separate PR
    /collision-dpo-tour/
  ],
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
