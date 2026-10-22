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

  it('empat kartu KPI periode berjalan dengan tren bertanda', async () => {
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
    expect($('[data-testid="kpi-expense"]').textContent).toContain('Periode 1 Okt – 31 Okt')
  })

  it('arus kas default 6 periode, bisa diganti 3', async () => {
    await view()
    const labels = () => $$('[data-testid="cashflow-card"] [data-part="x-label"]').length
    expect(labels()).toBe(6)
    expect($$('[data-testid="cashflow-card"] [data-part="area"]')).toHaveLength(2)
    const three = $$('[data-testid="cashflow-card"] button').find((b) => b.textContent!.includes('3 periode'))!
    three.click()
    await settle()
    expect(labels()).toBe(3)
    expect($('[data-testid="cashflow-card"] button[data-state="on"]').textContent).toContain('3 periode')
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
})
