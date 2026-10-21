import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import { useTransactionsStore } from '@/stores/transactions'
import { emptyForm } from '@/lib/txnForm'
import BudgetView from './BudgetView.vue'

const $ = <T extends Element = HTMLElement>(sel: string) => document.body.querySelector(sel) as T
const cats: Record<string, string> = {}
let bca = ''

async function settle() {
  await flushPromises()
  await nextTick()
  await flushPromises()
}

beforeAll(async () => {
  bca = await ipc.completeOnboarding({
    locale: 'id', periodStartDay: 1, accountKind: 'bank',
    account: { name: 'BCA', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 1_000_000, openingDate: '2026-09-01' },
  })
  for (const c of await ipc.listCategories()) cats[c.name] = c.id
})

async function view() {
  mount(BudgetView, { global: { plugins: [i18n] }, attachTo: document.body })
  await settle()
}

const key = (el: Element, k: string) => el.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true }))
const headers = () => [...document.body.querySelectorAll('[role="columnheader"]')].map((h) => h.textContent!.trim())
const cell = (name: string) => $(`[data-testid="budget-cell-${cats[name]}"]`)
const remaining = (name: string) => $(`[data-testid="budget-remaining-${cats[name]}"]`).textContent!.trim()

async function edit(name: string, text: string, key: string) {
  cell(name).click()
  await settle()
  const input = $<HTMLInputElement>(`[data-testid="budget-input-${cats[name]}"]`)
  input.value = text
  input.dispatchEvent(new Event('input', { bubbles: true }))
  input.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }))
  await settle()
}

describe('BudgetView', () => {
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

  it('menampilkan periode saat ini dan pindah ke periode sebelumnya', async () => {
    await view()
    expect($('[data-testid="budget-period"]').textContent).toBe('Okt 2026')
    expect(document.body.textContent).toContain('1 Okt – 31 Okt')
    $(`[aria-label="${id.budget.prev}"]`).click()
    await settle()
    expect($('[data-testid="budget-period"]').textContent).toBe('Sep 2026')
  })

  it('mengedit budget lewat sel lalu Enter menyimpan dan memperbarui sisa', async () => {
    await view()
    expect(cell('Transportasi').textContent).toContain('—')
    await edit('Transportasi', '300000', 'Enter')
    expect(cell('Transportasi').textContent).toContain('300.000')
    expect(remaining('Transportasi')).toBe('300.000')
    const o = await ipc.budgetOverview('2026-10')
    expect(o.rows.find((r) => r.categoryId === cats['Transportasi'])?.budget).toBe(300_000)
    await edit('Transportasi', '', 'Enter')
    expect(cell('Transportasi').textContent).toContain('—')
  })

  it('Esc membatalkan edit', async () => {
    await view()
    await edit('Tagihan', '999', 'Escape')
    expect($(`[data-testid="budget-input-${cats['Tagihan']}"]`)).toBeNull()
    expect(cell('Tagihan').textContent).toContain('—')
    expect((await ipc.budgetOverview('2026-10')).rows.find((r) => r.categoryId === cats['Tagihan'])?.budget).toBeNull()
  })

  it('sisa negatif berwarna destructive dengan tanda minus', async () => {
    await ipc.setBudget(cats['Kopi & jajan']!, '2026-10', 10_000)
    await ipc.createTxn({ kind: 'expense', date: '2026-10-02', accountId: bca, toAccountId: null, amount: 25_000,
      splits: [{ categoryId: cats['Kopi & jajan']!, amount: 25_000, memo: null }], fee: null, payee: null, note: null, tags: [] })
    await view()
    expect(remaining('Kopi & jajan')).toBe('−15.000')
    const row = $(`[data-testid="budget-row-${cats['Kopi & jajan']}"]`)
    expect(row.querySelector('[role="progressbar"]')!.getAttribute('data-state')).toBe('over')
    expect(row.querySelector('[data-testid^="budget-remaining"]')!.className).toContain('text-destructive')
  })

  it('salin dari bulan lalu mengisi budget kosong', async () => {
    await ipc.setBudget(cats['Restoran']!, '2026-09', 50_000)
    await view()
    expect(cell('Restoran').textContent).toContain('—')
    ;[...document.body.querySelectorAll('button')].find((b) => b.textContent?.includes(id.budget.copy))!.click()
    await settle()
    expect(cell('Restoran').textContent).toContain('50.000')
  })

  it('memuat ulang setelah transaksi disimpan dari form global', async () => {
    await view()
    expect(remaining('Kopi & jajan')).toBe('−15.000')
    await useTransactionsStore().save(null, {
      ...emptyForm('expense', { accountId: bca, feeCategoryId: null, today: '2026-10-05' }),
      amount: 5_000, categoryId: cats['Kopi & jajan']!,
    })
    await settle()
    expect(remaining('Kopi & jajan')).toBe('−20.000')
  })

  it('kolom per hari hanya untuk periode berjalan', async () => {
    await view()
    expect(headers()).toContain(id.budget.perDay)
    expect($('[data-testid="budget-total"]').textContent).toContain(id.budget.total)
    $(`[aria-label="${id.budget.next}"]`).click()
    await settle()
    expect(headers()).not.toContain(id.budget.perDay)
    expect(headers()).toContain(id.budget.carry)
  })

  it('rollover lewat menu baris menampilkan bawaan di periode berikutnya', async () => {
    await ipc.setBudget(cats['Biaya admin']!, '2026-10', 100_000)
    await ipc.setBudget(cats['Biaya admin']!, '2026-11', 50_000)
    await view()
    expect($(`[data-testid="budget-carry-${cats['Biaya admin']}"]`).textContent!.trim()).toBe('—')
    expect($(`[aria-label="${i18n.global.t('budget.rowMenu', { name: 'Tagihan' })}"]`)).toBeNull()
    key($(`[aria-label="${i18n.global.t('budget.rowMenu', { name: 'Biaya admin' })}"]`), 'Enter')
    await settle()
    const item = [...document.body.querySelectorAll('[role="menuitemcheckbox"]')].find((x) => x.textContent?.includes(id.budget.rollover)) as HTMLElement
    expect(item.getAttribute('aria-checked')).toBe('false')
    item.click()
    await settle()
    expect((await ipc.budgetOverview('2026-10')).rows.find((r) => r.categoryId === cats['Biaya admin'])?.rollover).toBe(true)
    $(`[aria-label="${id.budget.next}"]`).click()
    await settle()
    expect($(`[data-testid="budget-carry-${cats['Biaya admin']}"]`).textContent!.trim()).toBe('+100.000')
    expect(remaining('Biaya admin')).toBe('150.000')
  })

  it('ringkasan menghitung kategori hampir habis dan terlampaui', async () => {
    const a = await ipc.createCategory('expense', { name: 'Ringkas A', parentId: null, icon: null, color: null })
    const b = await ipc.createCategory('expense', { name: 'Ringkas B', parentId: null, icon: null, color: null })
    await ipc.setBudget(a, '2026-10', 100_000)
    await ipc.setBudget(b, '2026-10', 100_000)
    const spend = (c: string, amount: number) => ipc.createTxn({ kind: 'expense', date: '2026-10-03', accountId: bca, toAccountId: null,
      amount, splits: [{ categoryId: c, amount, memo: null }], fee: null, payee: null, note: null, tags: [] })
    await spend(a, 85_000)
    await spend(b, 120_000)
    const rows = (await ipc.budgetOverview('2026-10')).rows
    const warning = rows.filter((r) => r.level === 'warning').length
    const exceeded = rows.filter((r) => r.level === 'exceeded').length
    expect(warning).toBeGreaterThanOrEqual(1)
    expect(exceeded).toBeGreaterThanOrEqual(1)
    await view()
    const summary = $('[data-testid="budget-summary"]').textContent!
    expect(summary).toContain(i18n.global.t('budget.summaryWarning', { n: warning }))
    expect(summary).toContain(i18n.global.t('budget.summaryExceeded', { n: exceeded }))
  })
})
