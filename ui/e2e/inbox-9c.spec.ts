import { expect, type Page, test } from '@playwright/test'

// Inbox email capture (EML-03, Fase 9c). Draft disuntik lewat hook `__seedEmailInbox` browser-ledger.
const amount = (n: string) => new RegExp(`(^|[^\\d.])${n.replaceAll('.', '\\.')}$`)

type Seed = { id: string; date: string; amount: number; payee: string; category?: boolean }

async function onboard(page: Page) {
  await page.goto('/')
  await page.locator('#ob-name').fill('BCA')
  await page.locator('#ob-balance').fill('5000000')
  await page.getByRole('button', { name: 'Mulai pakai Nabungo' }).click()
  await expect(page.locator('[data-testid="kpi-net-worth"]')).toContainText('5.000.000')
}

async function go(page: Page, hash: string) {
  await page.evaluate((h) => { location.hash = h }, hash)
}

/** Suntik draft expense di akun BCA; kategori Tagihan bila `category`. Id diambil dari modul ipc dev server. */
async function seed(page: Page, captures: Seed[]) {
  await page.evaluate(async (list) => {
    const ipc = await import('/src/ipc/index.ts')
    const acc = (await ipc.accountsOverview()).accounts.find((a: { name: string }) => a.name === 'BCA')!.id
    const cat = (await ipc.listCategories()).find((c: { name: string }) => c.name === 'Tagihan')!.id
    const w = window as unknown as { __seedEmailInbox: (s: unknown) => void }
    w.__seedEmailInbox({ captures: list.map((c) => ({ id: c.id, kind: 'expense', date: c.date, amount: c.amount, payee: c.payee,
      accountId: acc, toAccountId: null, categoryId: c.category ? cat : null, ruleName: 'BCA QRIS', snippet: 'Transaksi QRIS berhasil' })) })
  }, captures)
  await go(page, '#/inbox')
}

const rows = (page: Page) => page.locator('[data-testid="capture-row"]')
const row = (page: Page, id: string) => page.locator(`[data-testid="capture-row"][data-capture-id="${id}"]`)

async function txnCount(page: Page) {
  await go(page, '#/transactions')
  await page.locator('[data-testid="txn-view-history"]').click()
  return page.locator('[data-txn-id]').count()
}

test('quick record shows in transactions', async ({ page }) => {
  await onboard(page)
  await seed(page, [{ id: 'q-1', date: '2026-05-09', amount: 41700, payee: 'Indomaret', category: true }])
  await row(page, 'q-1').locator('[data-testid="capture-record"]').click()
  await expect(rows(page)).toHaveCount(0)
  await go(page, '#/transactions')
  await expect(page.locator('[data-txn-id]').filter({ hasText: 'Indomaret' })).toBeVisible()
})

test('review with category then save', async ({ page }) => {
  await onboard(page)
  await seed(page, [{ id: 'r-1', date: '2026-05-09', amount: 41700, payee: 'Indomaret' }])
  await expect(row(page, 'r-1').locator('[data-testid="capture-record"]')).toHaveCount(0)
  await row(page, 'r-1').locator('[data-testid="capture-review"]').click()
  const sheet = page.getByRole('dialog')
  await expect(sheet).toContainText('Tinjau transaksi')
  await expect(page.locator('#txn-payee')).toHaveValue('Indomaret')
  await page.locator('#txn-category').click()
  await page.keyboard.type('Tagihan')
  await page.keyboard.press('Enter')
  await page.keyboard.press('Control+Enter')
  await expect(sheet).toBeHidden()
  await expect(rows(page)).toHaveCount(0)
  await go(page, '#/transactions')
  await expect(page.locator('[data-txn-id]').filter({ hasText: 'Indomaret' })).toBeVisible()
})

test('bulk ignore + undo', async ({ page }) => {
  await onboard(page)
  const dates = ['2026-05-01', '2026-05-02', '2026-05-03', '2026-05-04', '2026-05-05']
  await seed(page, dates.map((date, i) => ({ id: `b-${i}`, date, amount: 1000 * (i + 1), payee: `Toko ${i}` })))
  await expect(rows(page)).toHaveCount(5)
  await page.locator('[data-testid="inbox-menu"]').click()
  await page.getByRole('menuitem', { name: /Abaikan sampai/ }).click()
  await page.locator('#ignore-until-date').fill('2026-05-03')
  await page.locator('#ignore-until-date').press('Tab')
  await expect(page.locator('[data-testid="ignore-until-hint"]')).toContainText('3')
  await page.locator('[data-testid="ignore-until-submit"]').click()
  await expect(rows(page)).toHaveCount(2)
  await page.getByRole('button', { name: 'Urungkan' }).click()
  await expect(rows(page)).toHaveCount(5)
})

test('link duplicate', async ({ page }) => {
  await onboard(page)
  await page.locator('body').press('n')
  await page.locator('#txn-amount').fill('66000')
  await page.locator('#txn-category').click()
  await page.keyboard.type('Tagihan')
  await page.keyboard.press('Enter')
  await page.locator('#txn-payee').fill('Warung')
  await page.keyboard.press('Control+Enter')
  await expect(page.getByRole('dialog')).toBeHidden()
  const before = await txnCount(page)
  const today = await page.evaluate(() => {
    const d = new Date()
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
  })

  await seed(page, [{ id: 'd-1', date: today, amount: 66000, payee: 'Warung QRIS', category: true }])
  await row(page, 'd-1').locator('[data-testid="capture-duplicates"]').click()
  const dialog = page.getByRole('dialog')
  await expect(dialog).toContainText('Kemungkinan duplikat')
  await dialog.getByRole('button', { name: 'Hubungkan' }).click()
  await expect(dialog).toBeHidden()
  await expect(rows(page)).toHaveCount(0)
  expect(await txnCount(page)).toBe(before)
})

test('sidebar badge follows pending count', async ({ page }) => {
  await onboard(page)
  await expect(page.locator('[data-testid="nav-inbox-badge"]')).toHaveCount(0)
  await seed(page, [
    { id: 's-1', date: '2026-05-09', amount: 1000, payee: 'A', category: true },
    { id: 's-2', date: '2026-05-08', amount: 2000, payee: 'B', category: true },
  ])
  const badge = page.locator('[data-testid="nav-inbox-badge"]')
  await expect(badge).toHaveText('2')
  await row(page, 's-1').locator('[data-testid="capture-record"]').click()
  await expect(badge).toHaveText('1')
  await row(page, 's-2').locator('[data-testid="capture-menu"]').click()
  await page.getByRole('menuitem', { name: 'Abaikan' }).click()
  await expect(badge).toHaveCount(0)
})
