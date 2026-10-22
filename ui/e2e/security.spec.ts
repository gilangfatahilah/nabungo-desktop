import { expect, type Page, test } from '@playwright/test'

// Browser-ledger kosong di setiap page load; enkripsinya tiruan (tanpa kripto) dengan aturan yang sama dengan Rust.
const PW = 'rahasia-123'
const NEW_PW = 'password-baru-9'

async function go(page: Page, hash: string) {
  await page.evaluate((h) => { location.hash = h }, hash)
}

async function lockNow(page: Page) {
  // Kunci dari footer sidebar setelah Pengaturan tampil, agar redirect setelah buka kembali ke sini.
  await go(page, '#/settings')
  await expect(page).toHaveURL(/#\/settings/)
  await page.locator('[data-testid="sidebar-lock"]').click()
  await expect(page.getByRole('heading', { name: 'Nabungo terkunci' })).toBeVisible()
}

test('onboarding terenkripsi, kunci, buka dengan password lalu recovery key', async ({ page }) => {
  await page.goto('/')
  await page.locator('#ob-name').fill('BCA')
  await page.locator('#ob-balance').fill('1000000')
  await page.locator('[data-testid="ob-encrypt"]').click()
  await page.locator('#enc-password').fill(PW)
  await page.locator('#enc-confirm').fill(PW)
  await page.locator('#enc-ack').click()
  await page.getByRole('button', { name: 'Mulai pakai Nabungo' }).click()

  // Recovery key tampil sekali; "Selesai" baru aktif setelah dicentang.
  const dialog = page.getByRole('dialog')
  await expect(dialog.getByRole('heading', { name: 'Simpan recovery key' })).toBeVisible()
  const recoveryKey = (await dialog.locator('[data-testid="recovery-group"]').allTextContents()).join('-')
  expect(recoveryKey).toMatch(/^([0-9A-Z]{4}-){12}[0-9A-Z]{4}$/)
  const done = dialog.getByRole('button', { name: 'Selesai' })
  await expect(done).toBeDisabled()
  await page.keyboard.press('Escape')
  await expect(dialog).toBeVisible()
  await dialog.locator('#recovery-saved').click()
  await done.click()
  const netWorth = page.locator('[data-testid="sidebar-net-worth"]')
  await expect(netWorth).toContainText('1.000.000')

  // Password salah ditolak di field; password benar kembali ke halaman semula dengan data utuh.
  await lockNow(page)
  await expect(page).toHaveURL(/#\/unlock/)
  await page.locator('#unlock-password').fill('bukan-ini-123')
  await page.locator('#unlock-password').press('Enter')
  await expect(page.getByRole('alert')).toHaveText('Password salah')
  await page.locator('#unlock-password').fill(PW)
  await page.locator('#unlock-password').press('Enter')
  await expect(page).toHaveURL(/#\/settings/)
  await expect(netWorth).toContainText('1.000.000')

  // Recovery key (ditulis huruf kecil) + password baru → terbuka; password lama tidak berlaku lagi.
  await lockNow(page)
  await page.locator('[data-testid="unlock-use-recovery"]').click()
  await page.locator('#unlock-recovery').fill(recoveryKey.toLowerCase())
  await page.locator('#unlock-new-password').fill(NEW_PW)
  await page.locator('#unlock-confirm-password').fill(NEW_PW)
  await page.getByRole('button', { name: 'Pulihkan dan buka' }).click()
  await expect(netWorth).toContainText('1.000.000')

  await lockNow(page)
  await page.locator('#unlock-password').fill(PW)
  await page.locator('#unlock-password').press('Enter')
  await expect(page.getByRole('alert')).toHaveText('Password salah')
  await page.locator('#unlock-password').fill(NEW_PW)
  await page.locator('#unlock-password').press('Enter')
  await expect(netWorth).toContainText('1.000.000')
})
