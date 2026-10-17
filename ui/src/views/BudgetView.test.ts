import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
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

  it('sisa negatif ditandai bahaya', async () => {
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
})
