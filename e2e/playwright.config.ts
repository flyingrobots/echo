import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  retries: 0,
  use: {
    headless: true,
    viewport: { width: 1280, height: 800 },
    ignoreHTTPSErrors: true,
  },
})

