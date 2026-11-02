import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import { useTransactionsStore } from '@/stores/transactions'
import { emptyForm } from '@/lib/txnForm'
import ReportsView from './ReportsView.vue'

const $ = <T extends Element = HTMLElement>(sel: string) => document.body.querySelector(sel) as T
const $$ = (sel: string) => [...document.body.querySelectorAll<HTMLElement>(sel)]
const cells = (row: HTMLElement) => [...row.querySelectorAll<HTMLElement>('[role="cell"]')].map((c) => c.textContent!.trim())

let bca = ''

async function settle() {
  await flushPromises()
  await nextTick()
  await flushPromises()
}

async function view() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/reports', component: ReportsView }, { path: '/transactions', component: { template: '<div />' } }],
  })
  await router.push('/reports')
  await router.isReady()
  mount(ReportsView, { global: { plugins: [i18n, router] }, attachTo: document.body })
  await settle()
  return router
}

async function click(sel: string | HTMLElement) {
  ;(typeof sel === 'string' ? $(sel) : sel).click()
  await settle()
}

const openTable = (name: string) => click(`[data-testid="${name}-table-toggle"]`)
const names = () => $$('[data-testid="breakdown-row"] [data-part="name"]').map((n) => n.textContent!.trim())
const part = (el: Element, p: string) => el.querySelector(`[data-part="${p}"]`)?.textContent!.replace(/\s+/g, ' ').trim()

beforeAll(async () => {
  bca = await ipc.completeOnboarding({
    locale: 'id', periodStartDay: 1, accountKind: 'bank',
    account: { name: 'BCA', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 1_000_000, openingDate: '2026-09-01', contact: null },
  })
  const cats = await ipc.listCategories()
  const cat = (name: string) => cats.find((c) => c.name === name)!.id
  const txn = (kind: 'income' | 'expense', date: string, category: string, amount: number) =>
    ipc.createTxn({
      kind, date, accountId: bca, toAccountId: null, amount,
      splits: [{ categoryId: cat(category), amount, memo: null }], fee: null, payee: null, note: null, tags: [],
    })
  await txn('expense', '2026-09-10', 'Restoran', 50_000)
  await txn('income', '2026-10-01', 'Gaji', 8_000_000)
  await txn('expense', '2026-10-02', 'Kopi & jajan', 25_000)
  await txn('expense', '2026-10-03', 'Transportasi', 40_000)
})

describe('ReportsView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date(2026, 9, 17))
  })
  afterEach(() => {
    vi.useRealTimers()
    localStorage.clear()
    document.body.innerHTML = ''
  })

  it('KPI periode fokus dengan tren dan rasio tabungan', async () => {
    await view()
    expect($('[data-part="summary"]').textContent).toContain('1 Okt')
    expect($('[data-testid="period-label"]').textContent).toContain('Okt 2026')
    expect($<HTMLButtonElement>('[data-testid="period-next"]').disabled).toBe(true)
    const kpi = (k: string) => $(`[data-testid="reports-kpi-${k}"]`)
    expect(part(kpi('income'), 'amount')).toBe('Rp 8.000.000')
    expect(part(kpi('expense'), 'amount')).toBe('Rp 65.000')
    expect(part(kpi('expense'), 'trend')).toContain('+30%')
    expect(part(kpi('expense'), 'hint')).toBe('Rp 3.824 per hari')
    expect(part(kpi('savings'), 'amount')).toBe('99%')
  })

  it('arus kas: chart rentang, tabel terlipat, klik baris memindah fokus', async () => {
    await view()
    expect($$('[data-testid="cashflow-chart"] path[data-key]')).toHaveLength(24)
    expect($('[data-testid="cashflow-row-2026-10"]')).toBeNull()
    await openTable('cashflow')
    expect($$('[data-testid^="cashflow-row-"]')).toHaveLength(12)
    expect(cells($('[data-testid="cashflow-row-2026-10"]'))).toEqual(['Okt 2026', '8.000.000', '65.000', '7.935.000'])
    expect(cells($('[data-testid="cashflow-row-2026-09"]'))).toEqual(['Sep 2026', '0', '50.000', '−50.000'])
    expect($('[data-testid="cashflow-row-2026-09"] [data-part="net"]').className).toContain('text-destructive')

    await click('[data-testid="cashflow-row-2026-09"]')
    expect($('[data-testid="breakdown-title"]').textContent).toContain('Sep 2026')
    expect(names()).toEqual(['Makan & Minum'])
    expect($<HTMLButtonElement>('[data-testid="period-next"]').disabled).toBe(false)

    await click('[data-testid="period-next"]')
    expect($('[data-testid="breakdown-title"]').textContent).toContain('Okt 2026')
  })

  it('kategori: porsi, banding periode lalu, sub-kategori', async () => {
    await view()
    expect(names()).toEqual(['Transportasi', 'Makan & Minum'])
    const [transport, food] = $$('[data-testid="breakdown-row"]')
    expect(part(transport!, 'amount')).toBe('40.000')
    expect(part(transport!, 'share')).toBe('61,54%')
    expect(part(food!, 'trend')).toContain('−50%')
    expect($('[data-testid="breakdown-child"]')).toBeNull()
    await click(food!.querySelector<HTMLElement>('[data-part="expand"]')!)
    expect($$('[data-testid="breakdown-child"]').map((c) => c.textContent!.replace(/\s+/g, ' ').trim()))
      .toEqual([expect.stringContaining('Kopi & jajan'), expect.stringContaining('Restoran')])
  })

  it('tren kategori mengikuti pilihan baris; perubahan terbesar', async () => {
    await view()
    expect($('[data-testid="trend-category"]').textContent).toContain('Transportasi')
    await click($$('[data-testid="breakdown-row"] [data-part="select"]')[1]!)
    expect($('[data-testid="trend-category"]').textContent).toContain('Makan & Minum')
    expect(part($('[data-testid="reports-trend"]'), 'max')).toContain('50.000')
    await openTable('trend')
    expect($$('[data-testid="reports-trend"] [role="row"]')).toHaveLength(13)

    const changes = $('[data-testid="reports-changes"]')
    expect(part(changes.querySelector('[data-part="up"]')!, 'delta')).toBe('+40.000')
    expect(part(changes.querySelector('[data-part="down"]')!, 'delta')).toBe('−25.000')
  })

  it('panah kategori membuka Transaksi terfilter kategori dan periode', async () => {
    const router = await view()
    const transport = (await ipc.listCategories()).find((c) => c.name === 'Transportasi')!.id
    await click($('[data-testid="breakdown-row"] [data-part="view-txns"]'))
    expect(router.currentRoute.value.path).toBe('/transactions')
    const q = useTransactionsStore().query
    expect(q.categoryId).toBe(transport)
    expect([q.dateFrom, q.dateTo]).toEqual(['2026-10-01', '2026-10-31'])
  })

  it('tab pemasukan dan periode kosong', async () => {
    await view()
    const tab = $$('[role="tab"]').find((t) => t.textContent!.trim() === id.reports.income)!
    tab.dispatchEvent(new MouseEvent('mousedown', { button: 0, bubbles: true }))
    await settle()
    expect(names()).toEqual(['Gaji'])
    expect(part($('[data-testid="breakdown-row"]'), 'share')).toBe('100%')

    await click('[data-testid="period-prev"]')
    await click('[data-testid="period-prev"]')
    expect($$('[data-testid="breakdown-row"]')).toHaveLength(0)
    expect($('[data-testid="breakdown-empty"]').textContent).toContain(id.reports.emptyIncome)
  })

  it('rentang chart mengubah jumlah periode', async () => {
    await view()
    $('[data-testid="reports-range"]').dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
    await settle()
    const options = $$('[role="option"]')
    expect(options.map((o) => o.textContent!.trim())).toEqual(Object.values(id.reports.ranges))
    options[0]!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
    await settle()
    expect(localStorage.getItem('nabungo.reports.range')).toBe('3')
    expect($$('[data-testid="cashflow-chart"] path[data-key]')).toHaveLength(6)
  })

  it('kekayaan bersih menampilkan tabel per periode', async () => {
    await view()
    expect($$('[data-testid="networth-chart"] path[data-key]')).toHaveLength(3)
    expect($$('[data-testid="networth-chart"] [data-part="area"]')).toHaveLength(1)
    await openTable('networth')
    expect($$('[data-testid^="networth-row-"]')).toHaveLength(12)
    expect(cells($('[data-testid="networth-row-2026-08"]'))).toEqual(['Agu 2026', '0', '0', '0'])
    expect(cells($('[data-testid="networth-row-2026-09"]'))).toEqual(['Sep 2026', '950.000', '0', '950.000'])
    expect(cells($('[data-testid="networth-row-2026-10"]'))).toEqual(['Okt 2026', '8.885.000', '0', '8.885.000'])
  })

  it('memuat ulang setelah transaksi disimpan dari form global', async () => {
    await view()
    const transport = (await ipc.listCategories()).find((c) => c.name === 'Transportasi')!.id
    await useTransactionsStore().save(null, {
      ...emptyForm('expense', { accountId: bca, feeCategoryId: null, today: '2026-10-05' }),
      amount: 10_000, categoryId: transport,
    })
    await settle()
    await openTable('cashflow')
    expect(cells($('[data-testid="cashflow-row-2026-10"]'))).toEqual(['Okt 2026', '8.000.000', '75.000', '7.925.000'])
    expect(part($('[data-testid="breakdown-row"]'), 'share')).toBe('66,67%')
  })
})
