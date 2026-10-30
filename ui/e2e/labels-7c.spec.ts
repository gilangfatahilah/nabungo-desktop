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

async function go(page: Page, hash: string) {
  await page.evaluate((h) => { location.hash = h }, hash)
}

/** Expense 25rb kategori Kopi; opsional payee & tag. */
async function expense(page: Page, opts: { payee?: string; tag?: string } = {}) {
  await page.locator('body').press('n')
  const amountInput = page.locator('#txn-amount')
  await expect(amountInput).toBeFocused()
  await amountInput.pressSequentially('25rb')
  await page.keyboard.press('Tab')
  await page.keyboard.type('Kopi')
  await page.keyboard.press('Enter')
  if (opts.payee) await page.locator('#txn-payee').fill(opts.payee)
  if (opts.tag) {
    await page.locator('[data-testid="txn-more"]').click()
    await page.locator('#txn-tags').fill(opts.tag)
    await page.locator('#txn-tags').press('Enter')
  }
  await page.keyboard.press('Control+Enter')
  await expect(page.getByRole('dialog')).toBeHidden()
}

const labelRow = (page: Page, name: string) =>
  page.locator('[data-testid="label-row"]').filter({ has: page.getByText(name, { exact: true }) })

async function rowAction(page: Page, name: string, action: string) {
  const row = labelRow(page, name)
  await row.hover()
  await row.getByRole('button', { name: 'Aksi lain' }).click()
  await page.getByRole('menuitem', { name: action }).click()
}

test('kelola tag dan payee', async ({ page }) => {
  await onboard(page)
  await expense(page, { payee: 'Indomart', tag: 'kantor' })

  await go(page, '#/labels')
  await rowAction(page, '#kantor', 'Ubah')
  let dialog = page.getByRole('dialog')
  await dialog.locator('#label-name').fill('Kantor')
  await dialog.getByRole('button', { name: /Simpan/ }).click()
  await expect(dialog).toBeHidden()
  await expect(labelRow(page, '#Kantor')).toContainText('1 transaksi')

  await page.getByRole('tab', { name: 'Pihak' }).click()
  await page.getByRole('button', { name: 'Pihak baru' }).click()
  dialog = page.getByRole('dialog')
  await dialog.locator('#label-name').fill('Indomaret')
  await dialog.getByRole('button', { name: /Simpan/ }).click()
  await expect(dialog).toBeHidden()

  await rowAction(page, 'Indomart', 'Gabungkan…')
  dialog = page.getByRole('dialog')
  await dialog.locator('[data-testid="merge-target"]').click()
  await page.getByRole('option', { name: 'Indomaret' }).click()
  await dialog.getByRole('button', { name: 'Gabungkan' }).click()
  await expect(dialog).toBeHidden()
  await expect(labelRow(page, 'Indomart')).toHaveCount(0)
  await expect(labelRow(page, 'Indomaret')).toContainText('1 transaksi')

  await rowAction(page, 'Indomaret', 'Lihat transaksi')
  await expect(page.locator('[data-testid="chip-payee"]')).toContainText('Pihak: Indomaret')
  const rows = page.locator('[data-txn-id]')
  await expect(rows).toHaveCount(1)
  await expect(rows.first()).toContainText('Indomaret')
  await expect(rows.first().locator('[data-testid="txn-tag"]')).toHaveText('#Kantor')
})

test('lampiran simulasi', async ({ page }) => {
  await onboard(page)
  await expense(page)
  await go(page, '#/transactions')
  const row = page.locator('[data-txn-id]').filter({ hasText: 'Kopi' })
  await expect(row).toBeVisible()
  await expect(row.locator('[data-testid="txn-attachment"]')).toHaveCount(0)

  await row.dblclick()
  const sheet = page.getByRole('dialog')
  await sheet.locator('[data-testid="txn-more"]').click()
  await sheet.getByRole('button', { name: 'Lampirkan file' }).click()
  await expect(sheet.locator('[data-testid="attachment-row"]')).toContainText('struk-contoh.jpg')
  await page.keyboard.press('Escape')
  await expect(sheet).toBeHidden()
  await expect(row.locator('[data-testid="txn-attachment"]')).toBeVisible()

  // Dibuka lagi: seksi lampiran terbuka otomatis.
  await row.dblclick()
  await sheet.locator('[data-testid="attachment-remove"]').click()
  await page.getByRole('alertdialog').getByRole('button', { name: 'Hapus' }).click()
  await expect(sheet.locator('[data-testid="attachment-row"]')).toHaveCount(0)
  await page.keyboard.press('Escape')
  await expect(row.locator('[data-testid="txn-attachment"]')).toHaveCount(0)
})
