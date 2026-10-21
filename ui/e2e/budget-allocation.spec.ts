import { expect, type Page, test } from '@playwright/test'

// Browser-ledger kosong di setiap page load; tanggal transaksi = hari ini, jadi semua angka ada di periode berjalan.
async function onboard(page: Page) {
  await page.goto('/')
  await page.locator('#ob-name').fill('BCA')
  await page.locator('#ob-balance').fill('1000000')
  await page.getByRole('button', { name: 'Mulai pakai Nabungo' }).click()
  await expect(page.locator('[data-testid="sidebar-net-worth"]')).toBeVisible()
}

async function go(page: Page, hash: string) {
  await page.evaluate((h) => { location.hash = h }, hash)
}

async function quickExpense(page: Page) {
  await page.locator('body').press('n')
  const amount = page.locator('#txn-amount')
  await expect(amount).toBeFocused()
  await amount.pressSequentially('25rb')
  await page.keyboard.press('Tab')
  await expect(page.locator('#txn-category')).toBeFocused()
  await page.keyboard.type('Kopi')
  await page.keyboard.press('Enter')
  await page.keyboard.press('Control+Enter')
  await expect(page.getByRole('dialog')).toBeHidden()
}

async function transferToNewAccount(page: Page) {
  await go(page, '#/accounts')
  await page.getByRole('button', { name: 'Akun baru' }).first().click()
  await page.getByRole('dialog').locator('#acc-name').fill('Dompet')
  await page.getByRole('dialog').locator('button[type="submit"]').click()
  await page.locator('#acc-name').waitFor({ state: 'detached' })
  await page.locator('body').press('n')
  await page.locator('[data-testid="tab-transfer"]').click()
  await page.locator('#txn-amount').fill('200000')
  await page.locator('[data-testid="txn-to-account"]').click()
  await page.getByRole('option', { name: 'Dompet' }).click()
  await page.locator('#txn-amount').press('Control+Enter')
  await expect(page.getByRole('dialog')).toBeHidden()
}

test('budget follows expense', async ({ page }) => {
  await onboard(page)
  await go(page, '#/budget')
  const editLabel = 'Ubah budget Makan & Minum'
  const row = page.getByRole('row').filter({ has: page.getByRole('button', { name: editLabel }) })
  await page.getByRole('button', { name: editLabel }).click()
  await page.getByRole('textbox', { name: editLabel }).fill('100000')
  await page.keyboard.press('Enter')
  const remaining = row.locator('[data-testid^="budget-remaining-"]')
  await expect(remaining).toHaveText('100.000')

  await quickExpense(page)
  await go(page, '#/transactions')
  await expect(page.locator('[data-testid="filter-search"]')).toBeVisible()
  await go(page, '#/budget')
  await expect(row).toContainText('25.000')
  await expect(remaining).toHaveText('75.000')

  await transferToNewAccount(page)
  await go(page, '#/budget')
  await expect(remaining).toHaveText('75.000')
  await expect(page.locator('[data-testid="budget-total"]')).toContainText('25.000')
})

test('allocation shows actual share', async ({ page }) => {
  await onboard(page)
  await go(page, '#/allocation')
  await page.getByTestId('bucket-new').click()
  await page.locator('#bucket-name').fill('Liquid')
  await page.locator('#bucket-target').fill('100')
  await page.keyboard.press('Control+Enter')
  await expect(page.getByRole('dialog')).toBeHidden()

  await page.locator('[data-testid^="map-account-"]').first().click()
  await page.getByTestId('share-bucket-0').selectOption({ label: 'Liquid' })
  await page.getByTestId('share-percent-0').fill('100')
  await page.keyboard.press('Control+Enter')
  await expect(page.getByRole('dialog')).toBeHidden()

  const liquid = page.locator('[data-testid^="bucket-row-"]').filter({ hasText: 'Liquid' })
  await expect(liquid).toContainText('100%')
  await expect(liquid).toContainText('1.000.000')
  await expect(page.getByTestId('bucket-unallocated')).toHaveCount(0)
  await expect(page.getByText('Belum dipetakan')).toHaveCount(0)
})

test('reports show current period', async ({ page }) => {
  await onboard(page)
  await quickExpense(page)
  await go(page, '#/reports')
  const current = page.locator('[data-testid^="cashflow-row-"]').last()
  await expect(current).toHaveAttribute('aria-selected', 'true')
  await expect(current.getByRole('cell').nth(2)).toHaveText('25.000')
  await expect(page.getByTestId('breakdown-row')).toContainText('Makan & Minum')
})

test('budget alert and rollover', async ({ page }) => {
  await onboard(page)
  await go(page, '#/budget')
  const editLabel = 'Ubah budget Makan & Minum'
  await page.getByRole('button', { name: editLabel }).click()
  await page.getByRole('textbox', { name: editLabel }).fill('27000')
  await page.keyboard.press('Enter')
  await expect(page.locator('[data-testid^="budget-remaining-"]').first()).toHaveText('27.000')

  // 25rb dari 27rb = 92% → toast waspada.
  await quickExpense(page)
  await expect(page.getByText('Budget Makan & Minum sudah 92% terpakai')).toBeVisible()

  await go(page, '#/budget')
  await page.getByRole('button', { name: 'Aksi untuk Makan & Minum' }).click()
  await page.getByRole('menuitemcheckbox', { name: 'Bawa sisa ke periode berikutnya' }).click()
  await page.getByRole('button', { name: 'Periode berikutnya' }).click()
  // Bawaan hanya berlaku pada periode yang ber-budget.
  await page.getByRole('button', { name: editLabel }).click()
  await page.getByRole('textbox', { name: editLabel }).fill('10000')
  await page.keyboard.press('Enter')
  const row = page.getByRole('row').filter({ has: page.getByRole('button', { name: editLabel }) })
  await expect(row.locator('[data-testid^="budget-carry-"]')).toHaveText('+2.000')
  await expect(row.locator('[data-testid^="budget-remaining-"]')).toHaveText('12.000')
})
