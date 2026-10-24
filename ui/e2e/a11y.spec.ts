import { expect, type Page, test } from '@playwright/test'

// Aksesibilitas (Fase 8c): ukuran teks, kontras & ARIA (axe), navigasi keyboard. Berjalan di browser-ledger.

async function go(page: Page, hash: string) {
  await page.evaluate((h) => { location.hash = h }, hash)
}

async function onboard(page: Page) {
  await page.goto('/')
  await page.locator('#ob-name').fill('BCA')
  await page.locator('#ob-balance').fill('5000000')
  await page.getByRole('button', { name: 'Mulai pakai Nabungo' }).click()
  await expect(page.locator('[data-testid="sidebar-net-worth"]')).toBeVisible()
}

async function noHorizontalOverflow(page: Page) {
  const { scroll, client } = await page.evaluate(() => ({
    scroll: document.documentElement.scrollWidth,
    client: document.documentElement.clientWidth,
  }))
  expect(scroll).toBeLessThanOrEqual(client)
}

test('font scale 150 has no horizontal overflow', async ({ page }) => {
  await page.setViewportSize({ width: 960, height: 600 })
  await onboard(page)
  for (let i = 0; i < 3; i++) await page.locator('body').press('Control+Equal')
  await expect.poll(() => page.evaluate(() => document.documentElement.style.fontSize)).toBe('150%')

  for (const hash of ['#/', '#/transactions', '#/budget', '#/allocation', '#/reports', '#/accounts', '#/settings']) {
    await go(page, hash)
    await page.waitForLoadState('networkidle')
    await noHorizontalOverflow(page)
  }
  await go(page, '#/transactions')
  await page.locator('body').press('n')
  await expect(page.locator('#txn-amount')).toBeFocused()
  await noHorizontalOverflow(page)

  // Ukuran tersimpan dan dipakai lagi setelah muat ulang (cache sebelum DB terbuka).
  await page.keyboard.press('Escape')
  await page.locator('body').press('Control+0')
  await expect.poll(() => page.evaluate(() => document.documentElement.style.fontSize)).toBe('100%')
})
