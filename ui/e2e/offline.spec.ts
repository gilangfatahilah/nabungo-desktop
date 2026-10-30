import { expect, type Page, test } from '@playwright/test'

// Offline (PRD §8, Fase 8d): UI tidak pernah meminta origin selain dirinya sendiri (font, CDN, gambar).
// Bukti runtime binary asli ada di scripts/firewall-test.sh.

const SELF = 'http://localhost:5173'
const ROUTES = ['#/', '#/transactions', '#/budget', '#/allocation', '#/reports', '#/accounts', '#/categories', '#/labels', '#/labels?tab=payee', '#/scheduled', '#/settings']

async function go(page: Page, hash: string) {
  await page.evaluate((h) => { location.hash = h }, hash)
  await page.waitForLoadState('networkidle')
}

test('tidak ada request ke origin lain di semua layar', async ({ page }) => {
  const external: string[] = []
  page.on('request', (r) => {
    const u = new URL(r.url())
    if (['http:', 'https:', 'ws:', 'wss:'].includes(u.protocol) && u.origin !== SELF) external.push(r.url())
  })

  await page.goto('/')
  await page.locator('#ob-name').fill('BCA')
  await page.locator('#ob-balance').fill('5000000')
  await page.getByRole('button', { name: 'Mulai pakai Nabungo' }).click()
  await expect(page.locator('[data-testid="kpi-net-worth"]')).toBeVisible()

  await page.locator('body').press('n')
  await page.locator('#txn-amount').fill('25000')
  await page.locator('#txn-category').fill('Tagihan')
  await page.locator('#txn-category').press('Enter')
  await page.locator('#txn-payee').fill('Warung')
  await page.keyboard.press('Control+Enter')
  await expect(page.getByRole('dialog')).toBeHidden()

  for (const hash of ROUTES) await go(page, hash)
  await go(page, '#/transactions')
  await page.locator('[data-testid="txn-view-table"]').click()

  await page.keyboard.press('Control+k')
  await expect(page.getByRole('dialog')).toBeVisible()
  await page.keyboard.press('Escape')
  await page.waitForLoadState('networkidle')

  expect(external).toEqual([])
})
