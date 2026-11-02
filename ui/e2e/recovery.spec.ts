import { expect, type Page, test } from '@playwright/test'

// Mode pemulihan disimulasikan lewat hook `__enterRecovery` browser-ledger (P11).
async function go(page: Page, hash: string) {
  await page.evaluate((h) => { location.hash = h }, hash)
}

test('DB tidak terbaca: semua rute ke pemulihan, restore backup kembali ke dasbor', async ({ page }) => {
  await page.goto('/')
  await page.locator('#ob-name').fill('BCA')
  await page.locator('#ob-balance').fill('1000000')
  await page.getByRole('button', { name: 'Mulai pakai Nabungo' }).click()
  const netWorth = page.locator('[data-testid="kpi-net-worth"]')
  await expect(netWorth).toContainText('1.000.000')

  await go(page, '#/settings')
  await page.getByRole('button', { name: 'Backup sekarang…' }).click()
  await expect(page.getByText(/Backup disimpan/)).toBeVisible()

  await page.evaluate(() => (window as unknown as { __enterRecovery: (r: string) => void }).__enterRecovery('unreadable'))
  await go(page, '#/accounts')
  await expect(page).toHaveURL(/#\/recovery/)
  await expect(page.getByRole('heading', { name: 'Database tidak bisa dibuka' })).toBeVisible()
  await go(page, '#/')
  await expect(page).toHaveURL(/#\/recovery/)

  await page.locator('[data-testid="recovery-backup-row"]').first().getByRole('button', { name: 'Pulihkan' }).click()
  const dialog = page.getByRole('alertdialog')
  await expect(dialog.getByRole('heading', { name: 'Pulihkan dari backup ini?' })).toBeVisible()
  await dialog.getByRole('button', { name: 'Pulihkan' }).click()
  await expect(page).toHaveURL(/#\/$/)
  await expect(netWorth).toContainText('1.000.000')
  await go(page, '#/recovery')
  await expect(page).toHaveURL(/#\/$/)
})
