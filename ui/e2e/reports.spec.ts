import { expect, type Page, test } from '@playwright/test'

// Revamp Laporan: periode fokus, rentang chart, sub-kategori, tren kategori, drill-down ke Transaksi.
async function onboard(page: Page) {
  await page.goto('/')
  await page.locator('#ob-name').fill('BCA')
  await page.locator('#ob-balance').fill('1000000')
  await page.getByRole('button', { name: 'Mulai pakai Nabungo' }).click()
  await expect(page.locator('[data-testid="kpi-net-worth"]')).toBeVisible()
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

test('laporan: fokus periode, kategori, tren, drill-down', async ({ page }) => {
  await onboard(page)
  await quickExpense(page)
  await go(page, '#/reports')

  await expect(page.getByTestId('reports-kpi-expense').locator('[data-part="amount"]')).toContainText('25.000')
  const row = page.getByTestId('breakdown-row').filter({ hasText: 'Makan & Minum' })
  await expect(row.locator('[data-part="share"]')).toHaveText('100%')
  await row.locator('[data-part="expand"]').click()
  await expect(page.getByTestId('breakdown-child')).toContainText('Kopi & jajan')
  await expect(page.getByTestId('trend-category')).toContainText('Makan & Minum')

  // ‹ › tidak melewati periode berjalan.
  const label = page.getByTestId('period-label')
  const current = await label.textContent()
  await expect(page.getByTestId('period-next')).toBeDisabled()
  await page.getByTestId('period-prev').click()
  await expect(label).not.toHaveText(current!)
  await expect(page.getByTestId('breakdown-empty')).toBeVisible()
  await page.getByTestId('period-next').click()
  await expect(label).toHaveText(current!)

  await page.getByTestId('reports-range').click()
  await page.getByRole('option', { name: '3 bulan terakhir' }).click()
  await expect(page.locator('[data-testid="cashflow-chart"] [data-part="group"]')).toHaveCount(3)

  await row.locator('[data-part="view-txns"]').click()
  await expect(page).toHaveURL(/#\/transactions/)
  await expect(page.getByTestId('chip-category')).toContainText('Makan & Minum')
  await expect(page.getByTestId('filter-count')).toContainText('1')
})
