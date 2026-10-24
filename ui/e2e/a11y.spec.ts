import AxeBuilder from '@axe-core/playwright'
import { expect, type Page, test } from '@playwright/test'

// Aksesibilitas (Fase 8c): ukuran teks, kontras & ARIA (axe), navigasi keyboard. Berjalan di browser-ledger.

async function go(page: Page, hash: string) {
  await page.evaluate((h) => { location.hash = h }, hash)
}

async function onboard(page: Page) {
  await page.goto('/')
  await page.locator('#ob-name').fill('BCA')
  await page.locator('#ob-balance').fill('5000000')
  await page.getByRole('button', { name: 'Mulai pakai Nabungo' }).click()
  await expect(page.locator('[data-testid="sidebar-net-worth"]')).toBeVisible()
}

// Data contoh lewat UI: dua transaksi berpayee (payee + kategori + budget terisi).
async function seed(page: Page) {
  await onboard(page)
  for (const [amount, payee] of [['50000', 'Listrik'], ['25000', 'Warung']]) {
    await page.locator('body').press('n')
    await page.locator('#txn-amount').fill(amount)
    await page.locator('#txn-category').fill('Tagihan')
    await page.locator('#txn-category').press('Enter')
    await page.locator('#txn-payee').fill(payee)
    await page.keyboard.press('Control+Enter')
    await expect(page.getByRole('dialog')).toBeHidden()
  }
  await go(page, '#/budget')
  const editLabel = 'Ubah budget Makan & Minum'
  await page.getByRole('button', { name: editLabel }).click()
  await page.getByRole('textbox', { name: editLabel }).fill('100000')
  await page.keyboard.press('Enter')
}

async function expectNoViolations(page: Page, where: string) {
  // Tunggu animasi masuk (dialog, toast) selesai agar warna yang diukur adalah warna akhir.
  await page.waitForTimeout(250)
  const { violations } = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21aa'])
    // Toast yang sedang keluar / disembunyikan sonner (di luar `visibleToasts`) masih transparan sebagian:
    // warnanya bukan warna akhir. Toast yang tampil tetap di-scan (skenario "toast").
    .exclude('[data-sonner-toast][data-removed="true"]')
    .exclude('[data-sonner-toast][data-visible="false"]')
    .analyze()
  const report = violations.map((v) => `${v.id}: ${v.nodes.map((n) => `${n.target.join(' ')} (${n.any[0]?.message ?? n.failureSummary})`).join(' | ')}`)
  expect(report, where).toEqual([])
}

const ROUTES = ['#/', '#/transactions', '#/budget', '#/allocation', '#/reports', '#/accounts', '#/categories', '#/labels', '#/labels?tab=payee', '#/scheduled', '#/settings']

for (const scheme of ['light', 'dark'] as const) {
  test(`axe: semua layar tanpa pelanggaran (${scheme})`, async ({ page }) => {
    test.setTimeout(120_000)
    await page.emulateMedia({ colorScheme: scheme })
    await page.goto('/')
    await expectNoViolations(page, 'onboarding')
    await seed(page)

    for (const hash of ROUTES) {
      await go(page, hash)
      await page.waitForLoadState('networkidle')
      await expectNoViolations(page, hash)
    }
    await go(page, '#/transactions')
    await page.locator('[data-testid="txn-view-table"]').click()
    await expectNoViolations(page, 'transactions (tabel)')

    // Keadaan terbuka: sheet, tooltip chart, dialog konfirmasi, toast.
    await page.locator('body').press('n')
    await expect(page.locator('#txn-amount')).toBeFocused()
    await expectNoViolations(page, 'sheet transaksi')
    await page.keyboard.press('Escape')

    await go(page, '#/reports')
    await page.locator('[data-testid="cashflow-chart"] [role="group"]').focus()
    await expectNoViolations(page, 'tooltip chart')

    await go(page, '#/labels?tab=payee')
    const menu = page.locator('[data-testid^="label-menu-"]').first()
    await menu.click()
    await page.getByRole('menuitem', { name: 'Hapus' }).click()
    await expect(page.getByRole('alertdialog')).toBeVisible()
    await expectNoViolations(page, 'dialog konfirmasi')
    await page.keyboard.press('Escape')

    await go(page, '#/transactions')
    await page.locator('[data-testid="txn-view-history"]').click()
    const row = page.locator('[data-txn-id]').filter({ hasText: 'Warung' })
    await row.focus()
    await page.keyboard.press('Delete')
    await expect(page.getByRole('button', { name: 'Urungkan' })).toBeVisible()
    await expectNoViolations(page, 'toast')

    // Layar bare: kunci (enkripsi tiruan) dan pemulihan (hook browser-ledger).
    await go(page, '#/settings')
    await page.evaluate(() => (window as unknown as { __enterRecovery: (r: string) => void }).__enterRecovery('unreadable'))
    await go(page, '#/')
    await expect(page).toHaveURL(/#\/recovery/)
    await expectNoViolations(page, 'pemulihan')
  })
}

test('axe: layar kunci', async ({ page }) => {
  for (const scheme of ['light', 'dark'] as const) {
    await page.emulateMedia({ colorScheme: scheme })
    await page.goto('/')
    await page.locator('#ob-name').fill('BCA')
    await page.locator('#ob-balance').fill('1000000')
    await page.locator('[data-testid="ob-encrypt"]').click()
    await page.locator('#enc-password').fill('rahasia-123')
    await page.locator('#enc-confirm').fill('rahasia-123')
    await page.locator('#enc-ack').click()
    await page.getByRole('button', { name: 'Mulai pakai Nabungo' }).click()
    const dialog = page.getByRole('dialog')
    await expect(dialog.getByRole('heading', { name: 'Simpan recovery key' })).toBeVisible()
    await expectNoViolations(page, `recovery key (${scheme})`)
    await dialog.locator('#recovery-saved').click()
    await dialog.getByRole('button', { name: 'Selesai' }).click()
    await page.locator('[data-testid="sidebar-lock"]').click()
    await expect(page.getByRole('heading', { name: 'Nabungo terkunci' })).toBeVisible()
    await expectNoViolations(page, `kunci (${scheme})`)
  }
})

test('keyboard: Tab mencapai navigasi dan konten dengan fokus terlihat', async ({ page }) => {
  await seed(page)
  await go(page, '#/')
  await page.locator('body').click({ position: { x: 1, y: 1 } })
  const reached = new Set<string>()
  for (let i = 0; i < 40; i++) {
    await page.keyboard.press('Tab')
    const info = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null
      if (!el || el === document.body) return null
      const cs = getComputedStyle(el)
      const visible = (cs.outlineStyle !== 'none' && parseFloat(cs.outlineWidth) > 0) || cs.boxShadow !== 'none'
      return {
        id: el.dataset.testid ?? el.getAttribute('href') ?? el.textContent?.trim().slice(0, 30) ?? el.tagName,
        visible,
        sidebar: !!el.closest('[data-sidebar="sidebar"]'),
        main: !!el.closest('.\\@container\\/main'),
      }
    })
    if (!info) continue
    expect(info.visible, `fokus tak terlihat: ${info.id}`).toBe(true)
    if (info.sidebar) reached.add('sidebar')
    if (info.main) reached.add('main')
    if (info.id === 'dashboard-new-txn') reached.add('new-txn')
  }
  expect([...reached].sort()).toEqual(['main', 'new-txn', 'sidebar'])
})

test('keyboard: transaksi baru hanya dengan keyboard', async ({ page }) => {
  await onboard(page)
  await page.locator('body').press('n')
  await expect(page.locator('#txn-amount')).toBeFocused()
  await page.keyboard.type('15000')
  const category = page.locator('#txn-category')
  for (let i = 0; i < 10 && !(await category.evaluate((el) => el === document.activeElement)); i++) await page.keyboard.press('Tab')
  await expect(category).toBeFocused()
  await page.keyboard.type('Tagihan')
  await page.keyboard.press('Enter')
  await page.keyboard.press('Control+Enter')
  await expect(page.getByRole('dialog')).toBeHidden()
  await expect(page.locator('[data-testid="sidebar-net-worth"]')).toContainText('4.985.000')
})

// Konten berada di dalam inset `overflow-hidden` dengan scroller sendiri, jadi dokumen tidak pernah
// meluap; yang diperiksa adalah setiap scroller di dalam inset, judul SiteHeader yang terpotong, dan
// elemen fixed (toast) yang keluar dari jendela. Tabel data (`TableScroll`, role=region) boleh
// di-scroll sendiri: dikecualikan dari reflow oleh WCAG 1.4.10.
async function noHorizontalOverflow(page: Page, where: string) {
  const problems = await page.evaluate(() => {
    const out: string[] = []
    const inset = document.querySelector('[data-slot="sidebar-inset"]')
    for (const el of inset ? [inset, ...inset.querySelectorAll('*')] : []) {
      const cs = getComputedStyle(el)
      if (!/(auto|scroll)/.test(cs.overflowX) || el.getAttribute('role') === 'region') continue
      if (el.scrollWidth > el.clientWidth + 1) out.push(`scroll ${el.className.toString().slice(0, 40)} ${el.scrollWidth}>${el.clientWidth}`)
    }
    const h1 = document.querySelector('header h1') as HTMLElement | null
    if (h1 && h1.scrollWidth > h1.clientWidth + 1) out.push(`h1 terpotong "${h1.textContent}"`)
    for (const t of document.querySelectorAll('[data-sonner-toaster]')) {
      const r = t.getBoundingClientRect()
      if (r.width && (r.left < 0 || r.right > innerWidth)) out.push(`toaster ${Math.round(r.left)}..${Math.round(r.right)}`)
    }
    return out
  })
  expect.soft(problems, where).toEqual([])
}

test('font scale 150 has no horizontal overflow', async ({ page }) => {
  test.setTimeout(90_000)
  await page.setViewportSize({ width: 960, height: 600 })
  await seed(page)
  for (let i = 0; i < 3; i++) await page.locator('body').press('Control+Equal')
  await expect.poll(() => page.evaluate(() => document.documentElement.style.fontSize)).toBe('150%')

  for (const hash of ['#/', '#/transactions', '#/budget', '#/allocation', '#/reports', '#/accounts', '#/categories', '#/labels', '#/scheduled', '#/settings']) {
    await go(page, hash)
    await page.waitForTimeout(250)
    await noHorizontalOverflow(page, hash)
  }
  // Sheet terbuka + toast: simpan (toast "tersimpan" muncul) lalu langsung buka sheet lagi.
  // Toast digeser ke kiri sheet tapi tidak boleh keluar jendela.
  await go(page, '#/transactions')
  await page.locator('body').press('n')
  await expect(page.locator('#txn-amount')).toBeFocused()
  await page.keyboard.type('15000')
  await page.locator('#txn-category').click()
  await page.keyboard.type('Tagihan')
  await page.keyboard.press('Enter')
  await page.keyboard.press('Control+Enter')
  await expect(page.getByRole('dialog')).toBeHidden()
  await page.locator('body').press('n')
  await expect(page.locator('#txn-amount')).toBeFocused()
  await expect(page.locator('[data-sonner-toast]').first()).toBeVisible()
  await noHorizontalOverflow(page, 'sheet + toast')

  await page.keyboard.press('Escape')
  await page.locator('body').press('Control+0')
  await expect.poll(() => page.evaluate(() => document.documentElement.style.fontSize)).toBe('100%')
})
