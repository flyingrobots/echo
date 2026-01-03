// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

import { test, expect } from '@playwright/test'
import { resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

function fileUrl(rel: string) {
  return pathToFileURL(resolve(rel)).href
}

test.describe('Collision DPO Tour (static HTML)', () => {
  test('loads and renders', async ({ page }) => {
    await page.goto(fileUrl('docs/public/collision-dpo-tour.html'))
    await expect(page.locator('h1')).toHaveText(/Collision/i)
    // Animate script attaches pagers; ensure at least one exists
    await expect(page.locator('.pager').first()).toBeVisible()
  })

  test('tabs toggle World/Graph views', async ({ page }) => {
    await page.goto(fileUrl('docs/public/collision-dpo-tour.html'))
    // Find a figure with pip tabs
    const tabs = page.locator('.pip-tabs').first()
    await expect(tabs).toBeVisible()
    const graphTab = tabs.locator('.tab', { hasText: 'Graph' })
    const worldTab = tabs.locator('.tab', { hasText: 'World' })
    await graphTab.click()
    // Graph image should be visible, world hidden within the same figure
    const fig = tabs.locator('..') // pip
    const pip = fig
    await expect(pip.locator('img[alt="Graph view"]')).toBeVisible()
    await expect(pip.locator('img[alt="World view"]')).toBeHidden()
    await worldTab.click()
    await expect(pip.locator('img[alt="World view"]')).toBeVisible()
  })

  test('prev/next navigation toggles carousel mode', async ({ page }) => {
    await page.goto(fileUrl('docs/public/collision-dpo-tour.html'))
    const firstRule = page.locator('.rule').filter({ has: page.locator('.pager') }).first()
    await expect(firstRule).toBeVisible()
    const nextBtn = firstRule.locator('.pager .btn', { hasText: 'Next' }).first()
    await expect(nextBtn).toBeVisible()
    // Initially all slides are visible
    const figs = firstRule.locator('.step-grid figure')
    const total = await figs.count()
    expect(total).toBeGreaterThan(1)
    // Click next -> enter carousel mode (only one visible)
    await nextBtn.click()
    // Wait a tick for layout updates
    await page.waitForTimeout(50)
    const hiddenCount = await firstRule.locator('.step-grid figure.hidden').count()
    expect(hiddenCount).toBeGreaterThan(0)
  })
})
