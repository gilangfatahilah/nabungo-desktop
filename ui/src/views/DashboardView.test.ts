import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'
import { i18n } from '@/i18n'
import * as ipc from '@/ipc'
import { routes } from '@/router'
import { useTransactionsStore } from '@/stores/transactions'
import { useUiStore } from '@/stores/ui'
import DashboardView from './DashboardView.vue'

const $ = <T extends Element = HTMLElement>(sel: string) => document.body.querySelector(sel) as T
const $$ = (sel: string) => [...document.body.querySelectorAll<HTMLElement>(sel)]

let bca = ''
let kopi = ''
let dompet = ''

async function settle() {
  await flushPromises()
  await nextTick()
  await flushPromises()
}

async function view() {
  const router = createRouter({ history: createMemoryHistory(), routes })
  await router.push('/')
  mount(DashboardView, { global: { plugins: [i18n, router] }, attachTo: document.body })
  await settle()
}

beforeAll(async () => {
  bca = await ipc.completeOnboarding({
    locale: 'id', periodStartDay: 1, accountKind: 'bank',
    account: { name: 'BCA', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 1_000_000, openingDate: '2026-09-01', contact: null },
  })
  dompet = await ipc.createAccount('cash', { name: 'Dompet', groupId: null, includeInNetWorth: true, creditLimit: null,
    statementDay: null, icon: null, color: null, openingBalance: 0, openingDate: '2026-09-01', contact: null })
  const cats = await ipc.listCategories()
  const cat = (name: string) => cats.find((c) => c.name === name)!.id
  kopi = cat('Kopi & jajan')
  const txn = (kind: 'income' | 'expense', date: string, category: string, amount: number, payee: string | null = null) =>
    ipc.createTxn({
      kind, date, accountId: bca, toAccountId: null, amount,
      splits: [{ categoryId: cat(category), amount, memo: null }], fee: null, payee, note: null, tags: [],
    })
  await txn('expense', '2026-09-10', 'Restoran', 50_000)
  await txn('income', '2026-10-01', 'Gaji', 8_000_000)
  await txn('expense', '2026-10-02', 'Kopi & jajan', 25_000, 'Janji Jiwa')
  await txn('expense', '2026-10-03', 'Transportasi', 40_000)
  await ipc.setBudget(kopi, '2026-10', 20_000)
})

describe('DashboardView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date(2026, 9, 17))
  })
  afterEach(() => {
    vi.useRealTimers()
    document.body.innerHTML = ''
  })

  it('hero: kekayaan bersih dan tiga statistik periode berjalan dengan tren bertanda', async () => {
    await view()
    const amount = (k: string) => $(`[data-testid="kpi-${k}"] [data-part="amount"]`).textContent!.replace(/\s+/g, ' ')
    const trend = (k: string) => $(`[data-testid="kpi-${k}"] [data-part="trend"]`)?.textContent!.trim()
    expect(amount('remaining')).toBe('Rp 7.935.000')
    expect(amount('income')).toBe('Rp 8.000.000')
    expect(amount('expense')).toBe('Rp 65.000')
    expect(amount('net-worth')).toBe('Rp 8.885.000')
    // Pengeluaran 50.000 → 65.000; pemasukan periode lalu 0 → tanpa badge.
    expect(trend('expense')).toBe('+30%')
    expect(trend('income')).toBeUndefined()
    // Periode berjalan tampil sekali di PageHeader, bukan per statistik.
    expect(document.body.textContent).toContain('Periode 1 Okt – 31 Okt')
  })

  it('arus kas default 6 bulan, dropdown rentang bisa diganti ke 3 bulan', async () => {
    await view()
    const labels = () => $$('[data-testid="cashflow-card"] [data-part="x-label"]').length
    expect(labels()).toBe(6)
    expect($$('[data-testid="cashflow-card"] [data-part="area"]')).toHaveLength(2)
    const trigger = $('[data-testid="trend-range"]')
    expect(trigger.textContent).toContain('6 bulan terakhir')
    trigger.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
    await settle()
    const options = $$('[role="option"]')
    expect(options.map((o) => o.textContent!.trim())).toEqual(['3 bulan terakhir', '6 bulan terakhir', '1 tahun terakhir'])
    options[0]!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
    await settle()
    expect(labels()).toBe(3)
    expect($('[data-testid="trend-range"]').textContent).toContain('3 bulan terakhir')
  })

  it('transaksi terakhir terbaru di atas; klik membuka sheet edit', async () => {
    await view()
    const rows = $$('[data-testid="recent-txn"]')
    expect(rows.length).toBeGreaterThan(0)
    expect(rows[0]!.textContent).toContain('Transportasi')
    const kopiRow = rows.find((r) => r.textContent!.includes('Janji Jiwa'))!
    expect(kopiRow.textContent).toContain('Kopi & jajan')
    kopiRow.click()
    expect(useUiStore().txnEditor).not.toBeNull()
  })

  it('kartu Akun dan Budget perlu dilihat', async () => {
    await view()
    const account = $('[data-testid="dashboard-account"]')
    expect(account.textContent).toContain('BCA')
    expect(account.textContent).toMatch(/8\.885\.000$/)
    const watch = $$('[data-testid="budget-watch-row"]')
    expect(watch).toHaveLength(1)
    expect(watch[0]!.textContent).toContain('Kopi & jajan')
    expect(watch[0]!.textContent).toContain('125%')
    // Ada budget yang lewat batas: tidak ada keterangan "aman".
    expect($('[data-testid="budget-watch-safe"]')).toBeNull()
  })

  it('pilih akun: kartu detail mengikuti, aksi cepat membawa jenis dan akun', async () => {
    await view()
    const detail = () => $('[data-testid="account-detail"]').textContent!
    expect(detail()).toContain('BCA')
    expect(detail()).toContain('8.885.000')
    $$('[data-testid="dashboard-account"]').find((r) => r.textContent!.includes('Dompet'))!.click()
    await settle()
    expect(detail()).toContain('Dompet')
    expect($('[data-testid="dashboard-account"][aria-checked="true"]').textContent).toContain('Dompet')
    $('[data-testid="quick-transfer"]').click()
    expect(useUiStore().txnEditor).toEqual({ mode: 'create', preset: { kind: 'transfer', accountId: dompet } })
  })

  it('budget periode ini: terpakai dari tersedia', async () => {
    await view()
    const card = $('[data-testid="dashboard-budget-total"]')
    expect(card.textContent).toContain('125%')
    expect(card.textContent).toMatch(/25\.000.*20\.000/)
  })

  it('tren: tab kekayaan bersih satu seri', async () => {
    await view()
    $('[data-testid="trend-tab-netWorth"]').dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0 }))
    await settle()
    expect($$('[data-testid="cashflow-card"] [data-part="area"]')).toHaveLength(1)
    expect($$('[data-testid="cashflow-card"] [data-part="x-label"]')).toHaveLength(6)
  })

  it('pengeluaran periode ini: periode lalu dan donut per kategori', async () => {
    await view()
    const card = $('[data-testid="dashboard-expense"]')
    expect(card.querySelector('[data-part="current"]')!.textContent).toMatch(/65\.000/)
    expect(card.querySelector('[data-part="previous"]')!.textContent).toMatch(/50\.000/)
    const rows = $$('[data-testid="top-category-row"]').map((r) => r.textContent!)
    expect(rows[0]).toMatch(/Transportasi.*40\.000/)
    expect(rows[1]).toMatch(/Makan & Minum.*25\.000/)
    expect(card.querySelectorAll('path[data-key]').length).toBeGreaterThan(0)
    // Hover baris legenda menyorot segmen donut yang sama (tanpa tooltip: angkanya sudah di legenda).
    const transport = $$('[data-testid="top-category-row"]')[0]!
    transport.dispatchEvent(new MouseEvent('mouseenter'))
    await nextTick()
    const paths = [...card.querySelectorAll('path[data-key]')]
    expect(paths.filter((p) => p.getAttribute('opacity') === '0.4')).toHaveLength(paths.length - 1)
    expect(card.querySelector('[data-part="tooltip"]')).toBeNull()
    transport.dispatchEvent(new MouseEvent('mouseleave'))
    await nextTick()
    // Hover segmen donut langsung: tooltip nominal penuh.
    paths[0]!.dispatchEvent(new MouseEvent('mouseenter'))
    await nextTick()
    expect(card.querySelector('[data-part="tooltip"]')!.textContent).toMatch(/Transportasi.*Rp\s40\.000/)
  })

  it('budget perlu dilihat: keterangan aman bila semua budget level ok', async () => {
    await ipc.setBudget(kopi, '2026-10', 1_000_000)
    await view()
    expect($('[data-testid="budget-watch-safe"]').textContent).toContain('Semua budget masih aman')
    expect($$('[data-testid="budget-watch-row"]')).toHaveLength(1)
    await ipc.setBudget(kopi, '2026-10', 20_000)
  })

  it('memuat ulang setelah aksi tulis', async () => {
    await view()
    await ipc.createTxn({
      kind: 'expense', date: '2026-10-16', accountId: bca, toAccountId: null, amount: 10_000,
      splits: [{ categoryId: kopi, amount: 10_000, memo: null }], fee: null, payee: 'Warung', note: null, tags: [],
    })
    useTransactionsStore().ledgerVersion++
    await settle()
    expect($$('[data-testid="recent-txn"]')[0]!.textContent).toContain('Warung')
  })

  it('top kategori maksimal 5 sesuai urutan backend', async () => {
    // Default hanya punya 4 root pengeluaran: tambah 2 agar ada 6 root.
    await ipc.createCategory('expense', { name: 'Hiburan', parentId: null, icon: null, color: null })
    await ipc.createCategory('expense', { name: 'Kesehatan', parentId: null, icon: null, color: null })
    const roots = (await ipc.listCategories()).filter((c) => c.kind === 'expense' && !c.parentId && !c.archived)
    const picked = roots.slice(0, 6)
    for (const [i, c] of picked.entries()) {
      const amount = (6 - i) * 100_000
      await ipc.createTxn({
        kind: 'expense', date: '2026-10-04', accountId: bca, toAccountId: null, amount,
        splits: [{ categoryId: c.id, amount, memo: null }], fee: null, payee: null, note: null, tags: [],
      })
    }
    await view()
    const rows = $$('[data-testid="top-category-row"]')
    expect(rows).toHaveLength(5)
    expect(rows[0]!.textContent).toContain(picked[0]!.name)
    expect(rows[0]!.textContent).toMatch(/6\d\d\.000/)
  })

  it('kartu jatuh tempo maksimal 5 baris dan menaut ke Terjadwal', async () => {
    for (let i = 0; i < 6; i++) {
      await ipc.createRule({
        form: { kind: 'expense', date: '2026-10-10', accountId: bca, toAccountId: null, amount: 100_000 + i,
          splits: [{ categoryId: kopi, amount: 100_000 + i, memo: null }], fee: null, payee: `Tagihan ${i}`, note: null, tags: [] },
        recurrence: { freq: 'monthly', interval: 1, until: null }, autoPost: false,
      })
    }
    await view()
    expect($$('[data-testid="dashboard-due-row"]')).toHaveLength(5)
    const more = $<HTMLAnchorElement>('[data-testid="dashboard-due-more"]')
    expect(more.textContent).toContain('(6)')
    expect(more.getAttribute('href')).toContain('/scheduled')
  })

  it('ringkasan alokasi hanya menampilkan bucket root', async () => {
    const root = await ipc.createBucket({ name: 'Aset', parentId: null, targetBp: 10000, targetAmount: null, color: null })
    await ipc.createBucket({ name: 'Saham', parentId: root, targetBp: 10000, targetAmount: null, color: null })
    await view()
    const rows = $$('[data-testid="allocation-summary-row"]')
    expect(rows).toHaveLength(1)
    expect(rows[0]!.textContent).toContain('Aset')
    expect(rows[0]!.textContent).toContain('0% / target 100%')
    expect($('[data-testid="dashboard-allocation"]').textContent).toContain('Belum dialokasikan 100%')
  })
})

