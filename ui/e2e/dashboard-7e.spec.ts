import { expect, type Page, test } from '@playwright/test'

// Dasbor lengkap (RPT-01) dan target nominal bucket (CAP-01), Fase 7e.
const amount = (n: string) => new RegExp(`(^|[^\\d.])${n.replaceAll('.', '\\.')}$`)
const pad = (n: number) => String(n).padStart(2, '0')
const now = new Date()
const today = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`

async function go(page: Page, hash: string) {
  await page.evaluate((h) => { location.hash = h }, hash)
}

async function fillExpense(page: Page, value: string) {
  await page.locator('#txn-amount').fill(value)
  await page.locator('#txn-category').click()
  await page.keyboard.type('Tagihan')
  await page.keyboard.press('Enter')
  await page.locator('#txn-payee').fill('Listrik')
}

async function newBucket(page: Page, name: string, fill: () => Promise<void>) {
  await page.getByTestId('bucket-new').click()
  await page.locator('#bucket-name').fill(name)
  await fill()
  await page.keyboard.press('Control+Enter')
  await expect(page.getByRole('dialog')).toBeHidden()
}

test('dasbor lengkap dan bucket nominal', async ({ page }) => {
  await page.goto('/')
  await page.locator('#ob-name').fill('BCA')
  await page.locator('#ob-balance').fill('5000000')
  await page.getByRole('button', { name: 'Mulai pakai Nabungo' }).click()
  await expect(page.locator('[data-testid="kpi-net-worth"]')).toContainText('5.000.000')

  await page.locator('body').press('n')
  await expect(page.locator('#txn-amount')).toBeFocused()
  await fillExpense(page, '50000')
  await page.keyboard.press('Control+Enter')
  await expect(page.getByRole('dialog')).toBeHidden()

  // Jadwal bulanan mulai hari ini: kejadian pertama langsung jatuh tempo.
  await go(page, '#/scheduled')
  await page.locator('[data-testid="rule-new"]').click()
  await fillExpense(page, '150000')
  await page.locator('#txn-date').fill(today)
  await page.locator('#txn-date').press('Tab')
  await page.keyboard.press('Control+Enter')
  await expect(page.getByRole('dialog')).toBeHidden()
  await expect(page.locator('[data-testid="due-row"]')).toHaveCount(1)

  await go(page, '#/allocation')
  await newBucket(page, 'Dana darurat', async () => {
    await page.getByTestId('bucket-kind-amount').click()
    await page.locator('#bucket-amount').fill('2970000')
  })
  await newBucket(page, 'Investasi', () => page.locator('#bucket-target').fill('100'))
  const row = (name: string) => page.locator('[data-testid^="bucket-row-"]').filter({ hasText: name })
  await expect(page.getByText('Total aset 4.950.000')).toBeVisible()
  await expect(row('Dana darurat').locator('[data-testid^="bucket-target-amount-"]')).toContainText('2.970.000')
  await expect(row('Dana darurat').locator('[data-testid^="bucket-target-"]:not([data-testid*="amount"])')).toHaveText('60%')
  await expect(row('Investasi').locator('[data-testid^="bucket-target-"]')).toHaveText('40%')

  await go(page, '#/')
  await expect(page.locator('[data-testid="top-category-row"]')).toHaveCount(1)
  await expect(page.locator('[data-testid="dashboard-due-row"]')).toHaveCount(1)
  await expect(page.locator('[data-testid="allocation-summary-row"]')).toHaveCount(2)
  await expect(page.getByTestId('dashboard-allocation')).toContainText('0% / target 60%')
})
