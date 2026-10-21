import { expect, type Page, test } from '@playwright/test'

// Transaksi terjadwal (TRX-06, Fase 7d). Tanggal dihitung dari hari ini agar kejadian jatuh tempo stabil.
const amount = (n: string) => new RegExp(`(^|[^\\d.])${n.replaceAll('.', '\\.')}$`)
const pad = (n: number) => String(n).padStart(2, '0')
const iso = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
const now = new Date()
const firstOf = (monthsBack: number) => iso(new Date(now.getFullYear(), now.getMonth() - monthsBack, 1))

async function onboard(page: Page) {
  await page.goto('/')
  await page.locator('#ob-name').fill('BCA')
  await page.locator('#ob-balance').fill('5000000')
  await page.getByRole('button', { name: 'Mulai pakai Nabungo' }).click()
  await expect(page.locator('[data-testid="sidebar-net-worth"]')).toHaveText(amount('5.000.000'))
}

async function go(page: Page, hash: string) {
  await page.evaluate((h) => { location.hash = h }, hash)
}

/** Isi form expense di sheet yang sedang terbuka: nominal, kategori Tagihan, payee Listrik. */
async function fillExpense(page: Page, value: string) {
  const amountInput = page.locator('#txn-amount')
  await amountInput.fill(value)
  await page.locator('#txn-category').fill('Tagihan')
  await page.locator('#txn-category').press('Enter')
  await page.locator('#txn-payee').fill('Listrik')
}

async function newRule(page: Page, start: string) {
  await go(page, '#/scheduled')
  await page.locator('[data-testid="rule-new"]').click()
  const sheet = page.getByRole('dialog')
  await expect(sheet).toContainText('Jadwal baru')
  await fillExpense(page, '150000')
  await page.locator('#txn-date').fill(start)
  await page.locator('#txn-date').press('Tab')
  await expect(sheet.locator('[data-testid="recurrence-preview"] li')).toHaveCount(3)
  await page.keyboard.press('Control+Enter')
  await expect(sheet).toBeHidden()
}

const dueRows = (page: Page) => page.locator('[data-testid="due-row"]')

test('jadwal dengan mulai di masa lalu: catat, lewati, konfirmasi semua', async ({ page }) => {
  await onboard(page)
  await newRule(page, firstOf(3))
  await expect(dueRows(page)).toHaveCount(4)
  await expect(page.locator('[data-testid="nav-scheduled-badge"]')).toHaveText('4')

  await dueRows(page).first().locator('[data-testid="due-confirm"]').click()
  await expect(dueRows(page)).toHaveCount(3)
  await dueRows(page).first().locator('[data-testid="due-skip"]').click()
  await expect(dueRows(page)).toHaveCount(2)
  await dueRows(page).first().hover()
  await dueRows(page).first().locator('[data-testid="due-menu"]').click()
  await page.getByRole('menuitem', { name: 'Konfirmasi semua (2)' }).click()
  await expect(dueRows(page)).toHaveCount(0)
  await expect(page.locator('[data-testid="nav-scheduled-badge"]')).toHaveCount(0)

  await go(page, '#/transactions')
  await expect(page.locator('[data-txn-id]').filter({ hasText: 'Listrik' })).toHaveCount(3)
})

test('jadikan berulang dari transaksi', async ({ page }) => {
  await onboard(page)
  await page.locator('body').press('n')
  await expect(page.locator('#txn-amount')).toBeFocused()
  await fillExpense(page, '150000')
  await page.keyboard.press('Control+Enter')
  await expect(page.getByRole('dialog')).toBeHidden()

  await go(page, '#/transactions')
  const row = page.locator('[data-txn-id]').filter({ hasText: 'Listrik' })
  await row.hover()
  await row.locator('[data-testid="txn-menu"]').click()
  await page.locator('[data-testid="txn-menu-recurring"]').click()
  const sheet = page.getByRole('dialog')
  await expect(sheet).toContainText('Jadikan berulang')
  await expect(page.locator('#txn-amount')).toHaveValue('150000')
  await page.locator('#txn-amount').press('Control+Enter')
  await expect(sheet).toBeHidden()

  await go(page, '#/scheduled')
  const next = new Date(now.getFullYear(), now.getMonth() + 1, 1)
  const month = new Intl.DateTimeFormat('id-ID', { month: 'short' }).format(next)
  await expect(page.locator('[data-testid="rule-row"]')).toHaveCount(1)
  await expect(page.locator('[data-testid="rule-row"]')).toContainText(`Berikutnya:`)
  await expect(page.locator('[data-testid="rule-row"]')).toContainText(month)
  await expect(dueRows(page)).toHaveCount(0)
})

test('edit kejadian sebelum dicatat tidak mengubah template', async ({ page }) => {
  await onboard(page)
  await newRule(page, firstOf(0))
  await expect(dueRows(page)).toHaveCount(1)
  await dueRows(page).hover()
  await dueRows(page).locator('[data-testid="due-menu"]').click()
  await page.getByRole('menuitem', { name: 'Edit lalu catat…' }).click()
  const sheet = page.getByRole('dialog')
  await expect(sheet).toContainText('Catat kejadian')
  await expect(sheet.locator('[data-testid="recurrence-fields"]')).toHaveCount(0)
  await page.locator('#txn-amount').fill('175000')
  await page.locator('#txn-amount').press('Control+Enter')
  await expect(sheet).toBeHidden()
  await expect(dueRows(page)).toHaveCount(0)
  await expect(page.locator('[data-testid="rule-row"]')).toContainText('150.000')

  await go(page, '#/transactions')
  await expect(page.locator('[data-txn-id]').filter({ hasText: 'Listrik' })).toContainText('175.000')
})
