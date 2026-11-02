import { expect, type Page, test } from '@playwright/test'

// Mode sembunyikan nominal (docs/design/design.md §7.10): toggle global per perangkat; form tetap terlihat.

async function go(page: Page, hash: string) {
  await page.evaluate((h) => { location.hash = h }, hash)
  await page.waitForLoadState('networkidle')
}

// Backend mode browser in-memory: setiap reload kembali ke onboarding.
async function onboard(page: Page) {
  await page.locator('#ob-name').fill('BCA')
  await page.locator('#ob-balance').fill('5000000')
  await page.getByRole('button', { name: 'Mulai pakai Nabungo' }).click()
}

test('sembunyikan nominal di semua layar baca, tetap setelah reload, form tetap terlihat', async ({ page }) => {
  await page.goto('/')
  await onboard(page)
  const netWorth = page.locator('[data-testid="kpi-net-worth"] [data-part="amount"]')
  await expect(netWorth).toHaveText('Rp 5.000.000')

  await page.locator('header [data-testid="toggle-amounts"]').click()
  await expect(netWorth).toHaveText('Rp ••••••••')
  await expect(page.locator('header [data-testid="toggle-amounts"]')).toHaveAttribute('aria-pressed', 'true')

  for (const hash of ['#/', '#/accounts', '#/reports']) {
    await go(page, hash)
    await expect(page.locator('main')).not.toContainText('5.000.000')
  }

  await page.reload()
  await onboard(page)
  await expect(netWorth).toHaveText('Rp ••••••••')

  await page.locator('body').press('n')
  await page.locator('#txn-amount').fill('25000')
  await expect(page.locator('#txn-amount')).toHaveValue(/25/)
  await page.keyboard.press('Escape')

  await page.keyboard.press('Control+Shift+H')
  await expect(netWorth).toHaveText('Rp 5.000.000')
})
