import { expect, type Page, test } from '@playwright/test'

// Sidebar menampilkan angka tanpa prefiks Rp (docs/design/ui-guidelines.md §7.4).
const amount = (n: string) => new RegExp(`(^|[^\\d.])${n.replaceAll('.', '\\.')}$`)

async function onboard(page: Page) {
  await page.goto('/')
  await page.locator('#ob-name').fill('BCA')
  await page.locator('#ob-balance').fill('1000000')
  await page.getByRole('button', { name: 'Mulai pakai Nabungo' }).click()
  await expect(page.locator('[data-testid="sidebar-net-worth"]')).toHaveText(amount('1.000.000'))
}

const sidebarAccount = (page: Page, name: string) =>
  page.locator('[data-sidebar="menu-button"]').filter({ hasText: name })

async function goTransactions(page: Page) {
  await page.evaluate(() => { location.hash = '#/transactions' })
  await expect(page.locator('[data-testid="filter-search"]')).toBeVisible()
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

test('expense via keyboard', async ({ page }) => {
  await onboard(page)
  await quickExpense(page)
  await goTransactions(page)
  const row = page.locator('[data-txn-id]').filter({ hasText: 'Kopi' })
  await expect(row).toBeVisible()
  await expect(row.locator('[data-testid="txn-amount"]')).toHaveText(/[−-]Rp\s25\.000/)
  await expect(sidebarAccount(page, 'BCA')).toHaveText(amount('975.000'))
})

test('transfer with fee', async ({ page }) => {
  await onboard(page)
  await page.evaluate(() => { location.hash = '#/accounts' })
  await page.getByRole('button', { name: 'Akun baru' }).first().click()
  const dialog = page.getByRole('dialog')
  await dialog.locator('#acc-name').fill('Dompet')
  await dialog.locator('button[type="submit"]').click()
  await expect(sidebarAccount(page, 'Dompet')).toBeVisible()
  await page.locator('#acc-name').waitFor({ state: 'detached' })

  await page.locator('body').press('n')
  await page.locator('[data-testid="tab-transfer"]').click()
  await page.locator('#txn-amount').fill('200000')
  await page.locator('[data-testid="txn-to-account"]').click()
  await page.getByRole('option', { name: 'Dompet' }).click()
  await page.locator('[data-testid="fee-toggle"]').click()
  await page.getByLabel('Nominal biaya').fill('6500')
  await expect(page.locator('#txn-fee-category')).toHaveValue(/Biaya admin/)
  await page.locator('#txn-amount').press('Control+Enter')
  await expect(page.getByRole('dialog')).toBeHidden()

  await expect(sidebarAccount(page, 'BCA')).toHaveText(amount('793.500'))
  await expect(sidebarAccount(page, 'Dompet')).toHaveText(amount('200.000'))
})

test('delete and undo', async ({ page }) => {
  await onboard(page)
  await quickExpense(page)
  await goTransactions(page)
  const row = page.locator('[data-txn-id]').filter({ hasText: 'Kopi' })
  await row.focus()
  await page.keyboard.press('Delete')
  await expect(row).toHaveCount(0)
  await expect(sidebarAccount(page, 'BCA')).toHaveText(amount('1.000.000'))
  await page.getByRole('button', { name: 'Urungkan' }).click()
  await expect(row).toBeVisible()
  await expect(sidebarAccount(page, 'BCA')).toHaveText(amount('975.000'))
})
