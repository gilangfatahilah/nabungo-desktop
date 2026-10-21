import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
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
  mount(ReportsView, { global: { plugins: [i18n] }, attachTo: document.body })
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
    document.body.innerHTML = ''
  })

  it('menampilkan 12 periode dan tabel cashflow', async () => {
    await view()
    expect($('[data-part="summary"]').textContent).toContain(id.reports.lastPeriods.replace('{count}', '12'))
    expect($$('[data-testid="cashflow-chart"] rect[data-key]')).toHaveLength(24)
    const rows = $$('[data-testid^="cashflow-row-"]')
    expect(rows).toHaveLength(12)
    expect(cells($('[data-testid="cashflow-row-2026-10"]'))).toEqual(['Okt 2026', '8.000.000', '65.000', '7.935.000'])
    expect(cells($('[data-testid="cashflow-row-2026-09"]'))).toEqual(['Sep 2026', '0', '50.000', '−50.000'])
    expect($('[data-testid="cashflow-row-2026-09"] [data-part="net"]').className).toContain('text-destructive')
  })

  it('memilih periode memperbarui breakdown', async () => {
    await view()
    expect($('[data-testid="breakdown-title"]').textContent).toContain('Okt 2026')
    const names = () => $$('[data-testid="breakdown-row"] [data-part="name"]').map((n) => n.textContent!.trim())
    expect(names()).toEqual(['Transportasi', 'Makan & Minum'])
    expect(cells($$('[data-testid="breakdown-row"]')[0]!)).toEqual(['Transportasi', '40.000', '61,54%'])

    $('[data-testid="cashflow-row-2026-09"]').click()
    await settle()
    expect($('[data-testid="breakdown-title"]').textContent).toContain('Sep 2026')
    expect(names()).toEqual(['Makan & Minum'])

    const group = $$('[data-testid="cashflow-chart"] [data-part="group"]').at(-1)!
    group.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
    await settle()
    expect($('[data-testid="breakdown-title"]').textContent).toContain('Okt 2026')
  })

  it('tab pemasukan menampilkan kategori income', async () => {
    await view()
    const tab = $$('[role="tab"]').find((t) => t.textContent!.trim() === id.reports.income)!
    tab.dispatchEvent(new MouseEvent('mousedown', { button: 0, bubbles: true }))
    await settle()
    const rows = $$('[data-testid="breakdown-row"]')
    expect(rows.map(cells)).toEqual([['Gaji', '8.000.000', '100%']])

    $('[data-testid="cashflow-row-2026-08"]').click()
    await settle()
    expect($$('[data-testid="breakdown-row"]')).toHaveLength(0)
    expect($('[data-testid="breakdown-empty"]').textContent).toContain(id.reports.emptyIncome)
  })

  it('kekayaan bersih menampilkan tabel per periode', async () => {
    await view()
    expect($$('[data-testid="networth-chart"] polyline[data-key]')).toHaveLength(3)
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
    expect(cells($('[data-testid="cashflow-row-2026-10"]'))).toEqual(['Okt 2026', '8.000.000', '75.000', '7.925.000'])
    expect(cells($$('[data-testid="breakdown-row"]')[0]!)).toEqual(['Transportasi', '50.000', '66,67%'])
  })
})
