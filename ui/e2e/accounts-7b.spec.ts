import { expect, type Page, test } from '@playwright/test'

// Sidebar menampilkan angka tanpa prefiks Rp (docs/design/design.md §6.2).
const amount = (n: string) => new RegExp(`(^|[^\\d.])${n.replaceAll('.', '\\.')}$`)

async function onboard(page: Page) {
  await page.goto('/')
  await page.locator('#ob-name').fill('BCA')
  await page.locator('#ob-balance').fill('1000000')
  await page.getByRole('button', { name: 'Mulai pakai Nabungo' }).click()
  await expect(page.locator('[data-testid="kpi-net-worth"]')).toContainText('1.000.000')
}

async function goAccounts(page: Page) {
  await page.evaluate(() => { location.hash = '#/accounts' })
  await expect(page.locator('[data-testid="account-row"]').first()).toBeVisible()
}

const accountRow = (page: Page, name: string) => page.locator('[data-testid="account-row"]').filter({ hasText: name })

test('piutang/hutang dengan kontak', async ({ page }) => {
  await onboard(page)
  await goAccounts(page)
  await page.getByRole('button', { name: 'Akun baru' }).first().click()
  const dialog = page.getByRole('dialog')
  await dialog.locator('#acc-name').fill('Pinjaman Budi')
  await dialog.locator('[data-testid="kind"]').click()
  await page.getByRole('option', { name: 'Piutang/Hutang' }).click()
  await dialog.locator('#acc-contact').fill('Budi')
  await dialog.locator('[data-testid="direction-iOwe"]').click()
  await dialog.locator('#acc-opening').fill('50000')
  await dialog.locator('button[type="submit"]').click()
  await expect(dialog).toBeHidden()

  const row = accountRow(page, 'Pinjaman Budi')
  await expect(row).toContainText('Hutang · Budi')
  await expect(row).toContainText(/Rp\s50\.000/)
  await expect(row).not.toContainText('−')
  // Hutang dihitung sebagai liabilitas: kekayaan bersih turun 50.000.
  await expect(page.locator('[data-testid="kpi-net-worth"]')).toContainText('950.000')
})

test('rekonsiliasi akun bank', async ({ page }) => {
  await onboard(page)
  await goAccounts(page)
  const row = accountRow(page, 'BCA')
  await row.hover()
  await row.getByRole('button', { name: 'Aksi lain' }).click()
  await page.locator('[data-testid="account-reconcile"]').click()
  const dialog = page.getByRole('dialog')
  await expect(dialog).toContainText('Saldo tercatat per tanggal ini')
  await dialog.locator('#rec-balance').fill('990000')
  await dialog.getByRole('button', { name: 'Rekonsiliasi' }).click()
  await expect(dialog).toBeHidden()
  await expect(page.getByText(/Penyesuaian −Rp\s10\.000 dicatat/)).toBeVisible()
  await expect(row).toContainText(/Rp\s990\.000/)
  await expect(row).toContainText('Direkonsiliasi s.d.')

  await page.evaluate(() => { location.hash = '#/transactions' })
  await page.locator('[data-testid="filter-account"]').click()
  await page.getByRole('option', { name: 'BCA' }).click()
  const adj = page.locator('[data-txn-id]').filter({ has: page.getByRole('img', { name: 'Penyesuaian' }) })
  await expect(adj).toBeVisible()
  await expect(adj.locator('[data-testid="txn-reconciled"]')).toBeVisible()
})
