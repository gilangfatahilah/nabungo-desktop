import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import TxnTable from '@/components/txn/TxnTable.vue'
import { i18n } from '@/i18n'
import * as ipc from '@/ipc'
import { useTransactionsStore } from '@/stores/transactions'
import { useUiStore } from '@/stores/ui'

let bca = ''
let dompet = ''
let kopiCat = ''
let transferId = ''
let expenseId = ''

beforeAll(async () => {
  bca = await ipc.completeOnboarding({
    locale: 'id', periodStartDay: 1, accountKind: 'bank',
    account: { name: 'BCA', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 1_000_000, openingDate: '2026-10-01', contact: null },
  })
  dompet = await ipc.createAccount('cash', { name: 'Dompet', groupId: null, includeInNetWorth: true, creditLimit: null,
    statementDay: null, icon: null, color: null, openingBalance: 0, openingDate: '2026-10-01', contact: null })
  kopiCat = (await ipc.listCategories()).find((c) => c.name === 'Kopi & jajan')!.id
  transferId = (await ipc.createTxn({ kind: 'transfer', date: '2026-10-03', accountId: bca, toAccountId: dompet,
    amount: 200_000, splits: [], fee: null, payee: null, note: null, tags: [] })).id
  expenseId = (await ipc.createTxn({ kind: 'expense', date: '2026-10-04', accountId: bca, toAccountId: null,
    amount: 25_000, splits: [{ categoryId: kopiCat, amount: 25_000, memo: null }], fee: null,
    payee: 'Kopi Kenangan', note: null, tags: ['ngopi'] })).id
})

async function settle() {
  await flushPromises()
  await nextTick()
  await flushPromises()
}

async function mountTable() {
  const w = mount(TxnTable, { props: { virtual: false }, global: { plugins: [i18n] }, attachTo: document.body })
  await settle()
  return w
}

const rowEl = (id: string) => document.body.querySelector<HTMLElement>(`[data-txn-id="${id}"]`)!

describe('TxnTable', () => {
  beforeEach(async () => {
    document.body.innerHTML = ''
    localStorage.clear()
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
    await useTransactionsStore().reload()
  })

  it('baris: tanggal, deskripsi, kategori, akun asal → tujuan, tag, nominal bertanda', async () => {
    await mountTable()
    const exp = rowEl(expenseId)
    expect(exp.textContent).toContain('4 Okt 2026')
    expect(exp.textContent).toContain('Kopi Kenangan')
    expect(exp.querySelector('[data-testid="cell-category"]')!.textContent).toContain('Kopi & jajan')
    expect(exp.querySelector('[data-testid="cell-filter-tag"]')!.textContent).toBe('#ngopi')
    expect(exp.querySelector('[data-testid="txn-amount"]')!.className).toContain('text-expense')
    const tr = rowEl(transferId)
    expect([...tr.querySelectorAll('[data-testid="cell-filter-account"]')].map((b) => b.textContent)).toEqual(['BCA', 'Dompet'])
    expect(tr.querySelector('[data-testid="cell-filter-category"]')).toBeNull()
  })

  it('klik header memanggil setSort; aria-sort mengikuti urutan aktif', async () => {
    const w = await mountTable()
    const store = useTransactionsStore()
    const th = () => w.find('[data-testid="sort-amount"]').element.closest('th')!
    expect(th().getAttribute('aria-sort')).toBe('none')
    await w.find('[data-testid="sort-amount"]').trigger('click')
    await settle()
    expect(store.sort).toEqual({ field: 'amount', desc: true })
    expect(th().getAttribute('aria-sort')).toBe('descending')
    expect(store.rows[0]!.kind).toBe('opening')
    await w.find('[data-testid="sort-amount"]').trigger('click')
    await settle()
    expect(th().getAttribute('aria-sort')).toBe('ascending')
    expect(store.rows[0]!.id).toBe(expenseId)
  })

  it('kolom saldo hanya saat filter satu akun dan urut tanggal', async () => {
    const store = useTransactionsStore()
    const w = await mountTable()
    expect(w.find('[data-testid="col-balance"]').exists()).toBe(false)
    await store.setQuery({ accountId: bca })
    await settle()
    expect(w.find('[data-testid="col-balance"]').exists()).toBe(true)
    expect(rowEl(expenseId).querySelector('[data-testid="txn-running"]')!.textContent).toContain('775.000')
    await store.setSort('amount')
    await settle()
    expect(w.find('[data-testid="col-balance"]').exists()).toBe(false)
  })

  it('klik akun/kategori/tag memfilter tanpa membuka editor', async () => {
    await mountTable()
    const store = useTransactionsStore()
    const spy = vi.spyOn(store, 'setQuery').mockResolvedValue()
    const exp = rowEl(expenseId)
    exp.querySelector<HTMLElement>('[data-testid="cell-filter-category"]')!.click()
    exp.querySelector<HTMLElement>('[data-testid="cell-filter-tag"]')!.click()
    rowEl(transferId).querySelectorAll<HTMLElement>('[data-testid="cell-filter-account"]')[1]!.click()
    expect(spy.mock.calls.map((c) => c[0])).toEqual([{ categoryId: kopiCat }, { tag: 'ngopi' }, { accountId: dompet }])
    expect(useUiStore().txnEditor).toBeNull()
  })

  it('klik baris membuka editor; Enter juga', async () => {
    await mountTable()
    rowEl(expenseId).click()
    expect(useUiStore().txnEditor).toEqual({ mode: 'edit', id: expenseId })
  })

  it('panah bawah memindah fokus ke baris berikutnya', async () => {
    await mountTable()
    const store = useTransactionsStore()
    const first = rowEl(store.rows[0]!.id)
    first.focus()
    first.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true }))
    await settle()
    expect(document.activeElement).toBe(rowEl(store.rows[1]!.id))
  })

  it('kolom tersembunyi tidak dirender', async () => {
    const store = useTransactionsStore()
    store.toggleColumn('tags', false)
    store.toggleColumn('account', false)
    await mountTable()
    expect(rowEl(expenseId).querySelector('[data-testid="cell-filter-tag"]')).toBeNull()
    expect(rowEl(expenseId).querySelector('[data-testid="cell-account"]')).toBeNull()
    expect(rowEl(expenseId).querySelector('[data-testid="cell-category"]')).not.toBeNull()
  })
})
