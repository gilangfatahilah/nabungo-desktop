import { expect, type Page, test } from '@playwright/test'

// Rule editor email capture (EML-04, Fase 9d). Email dihubungkan lewat browser-email (menyalin rule bawaan),
// email gagal dibaca disuntik lewat `__seedEmailInbox`; re-parse berjalan lokal tanpa sync.
const amount = (n: string) => new RegExp(`(^|[^\\d.])${n.replaceAll('.', '\\.')}$`)

type Unparsed = { messageId: string; sender: string; subject: string; receivedAt: string | null; snippet: string }

const BCA_QRIS: Unparsed = {
  messageId: '<qris-9d@bca>', sender: 'bca@bca.co.id', subject: 'Internet Transaction Journal', receivedAt: '2026-05-09T05:00:00Z',
  snippet: 'Transaction Date : 09 May 2026 12:00:00\nPayment to : IDM INDOMARET\nTotal Payment : IDR 41,700.00\n',
}

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

/** Suntik email gagal dibaca lalu hubungkan email (rule bawaan disalin); `reparse` menjalankan rule aktif sekali. */
async function setup(page: Page, unparsed: Unparsed[], opts: { reparse?: boolean; tab?: string } = {}) {
  await onboard(page)
  await page.evaluate(async ({ list, reparse }) => {
    const w = window as unknown as { __seedEmailInbox: (s: unknown) => void }
    w.__seedEmailInbox({ unparsed: list })
    const ipc = await import('/src/ipc/index.ts')
    await ipc.connectEmail('saya@gmail.com', 'Finance', 'rahasia')
    if (reparse) await ipc.reparseUnparsed()
  }, { list: unparsed, reparse: opts.reparse ?? false })
  await go(page, `#/inbox${opts.tab ? `?tab=${opts.tab}` : ''}`)
}

const ruleRow = (page: Page, name: string) => page.locator('[data-testid="rule-row"]').filter({ hasText: name })
const sheet = (page: Page) => page.getByRole('dialog')

async function pick(page: Page, testid: string, option: string) {
  await page.locator(`[data-testid="${testid}"]`).click()
  await page.getByRole('option', { name: option, exact: true }).click()
}

test('set account makes Catat appear', async ({ page }) => {
  await setup(page, [BCA_QRIS], { reparse: true })
  const row = page.locator('[data-testid="capture-row"]')
  await expect(row).toHaveCount(1)
  await expect(row.locator('[data-testid="capture-record"]')).toHaveCount(0)

  await page.getByRole('tab', { name: /^Rule/ }).click()
  await expect(page).toHaveURL(/tab=rules/)
  await ruleRow(page, 'BCA Pembayaran (QRIS/VA)').locator('[data-part="title"]').click()
  await expect(sheet(page)).toContainText('Ubah rule')
  await pick(page, 'er-account', 'BCA')
  await pick(page, 'er-category', 'Tagihan')
  await sheet(page).getByRole('button', { name: 'Simpan', exact: true }).click()
  await expect(sheet(page)).toBeHidden()
  await expect(ruleRow(page, 'BCA Pembayaran (QRIS/VA)')).toContainText('Tagihan')

  await page.getByRole('tab', { name: /Perlu ditinjau/ }).click()
  await expect(row.locator('[data-testid="capture-record"]')).toBeVisible()
})

test('create rule from unparsed', async ({ page }) => {
  await setup(page, [{ messageId: '<toko-9d@x>', sender: 'notif@tokobaru.example', subject: 'Pembayaran Berhasil',
    receivedAt: '2026-05-09T03:00:00Z', snippet: 'Anda membayar Rp 15.000 di toko 7' }], { tab: 'unparsed' })
  await page.locator('[data-testid="unparsed-create-rule"]').click()
  await expect(sheet(page)).toContainText('Rule baru')
  await expect(page.locator('#er-sender')).toHaveValue('tokobaru.example')
  await page.getByRole('button', { name: 'Uji', exact: true }).click()
  await expect(page.locator('[data-testid="rule-test-items"]')).toContainText('15.000')
  await expect(page.locator('[data-step="amount"]')).toHaveAttribute('data-ok', 'true')

  await pick(page, 'er-account', 'BCA')
  await sheet(page).getByRole('button', { name: 'Simpan', exact: true }).click()
  await expect(page.getByText('Rule disimpan · 1 email dipindah ke Perlu ditinjau')).toBeVisible()
  await expect(page.locator('[data-testid="unparsed-row"]')).toHaveCount(0)

  await page.getByRole('tab', { name: /Perlu ditinjau/ }).click()
  const row = page.locator('[data-testid="capture-row"]')
  await expect(row).toHaveCount(1)
  await expect(row.locator('[data-testid="capture-record"]')).toHaveCount(0)
  // Kategori dipilih lewat rule: draft yang sudah ada ikut siap tanpa sync.
  await page.getByRole('tab', { name: /^Rule/ }).click()
  await ruleRow(page, 'Pembayaran Berhasil').locator('[data-part="title"]').click()
  await pick(page, 'er-category', 'Tagihan')
  await sheet(page).getByRole('button', { name: 'Simpan', exact: true }).click()
  await expect(sheet(page)).toBeHidden()
  await page.getByRole('tab', { name: /Perlu ditinjau/ }).click()
  await row.locator('[data-testid="capture-record"]').click()
  await expect(row).toHaveCount(0)
})

test('invalid regex blocked', async ({ page }) => {
  await setup(page, [], { tab: 'rules' })
  await expect(page.locator('[data-testid="rule-row"]')).toHaveCount(7)
  await page.locator('[data-testid="rule-new"]').click()
  await page.locator('#er-name').fill('Rusak')
  await page.locator('#er-sender').fill('rusak.example')
  await page.locator('#er-amount').fill('(')
  await sheet(page).getByRole('button', { name: 'Simpan', exact: true }).click()
  await expect(page.locator('[data-testid="er-error-amount"]')).toBeVisible()
  await expect(page.locator('#er-amount')).toHaveAttribute('aria-invalid', 'true')
  await expect(sheet(page)).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(page.locator('[data-testid="rule-row"]')).toHaveCount(7)
})

test('disable rule then enable reparses', async ({ page }) => {
  await setup(page, [BCA_QRIS], { tab: 'rules' })
  const rule = ruleRow(page, 'BCA Pembayaran (QRIS/VA)')
  const toggle = rule.getByRole('switch')
  await toggle.click()
  await expect(rule).toContainText('Nonaktif')
  await expect(toggle).toHaveAttribute('aria-checked', 'false')
  await toggle.click()
  await expect(page.getByText('Rule disimpan · 1 email dipindah ke Perlu ditinjau')).toBeVisible()
  await expect(rule).not.toContainText('Nonaktif')
  await page.getByRole('tab', { name: /Perlu ditinjau/ }).click()
  await expect(page.locator('[data-testid="capture-row"]')).toHaveCount(1)
})

test('delete rule + undo', async ({ page }) => {
  await setup(page, [], { tab: 'rules' })
  const rows = page.locator('[data-testid="rule-row"]')
  await expect(rows).toHaveCount(7)
  const rule = ruleRow(page, 'BCA Tarik Tunai')
  await rule.hover()
  await rule.locator('[data-testid="rule-menu"]').click()
  await page.getByRole('menuitem', { name: 'Hapus' }).click()
  const confirm = page.getByRole('alertdialog')
  await expect(confirm).toContainText('BCA Tarik Tunai')
  await confirm.getByRole('button', { name: 'Hapus' }).click()
  await expect(rows).toHaveCount(6)
  await page.getByRole('button', { name: 'Urungkan' }).click()
  await expect(rows).toHaveCount(7)
  await expect(ruleRow(page, 'BCA Tarik Tunai')).toBeVisible()
})
