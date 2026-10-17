import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import TxnList from '@/components/txn/TxnList.vue'
import { i18n } from '@/i18n'
import * as ipc from '@/ipc'
import { useTransactionsStore } from '@/stores/transactions'
import { useUiStore } from '@/stores/ui'

let bca = ''
let dompet = ''
let transferId = ''
let expenseId = ''

beforeAll(async () => {
  bca = await ipc.completeOnboarding({
    locale: 'id', periodStartDay: 1, accountKind: 'bank',
    account: { name: 'BCA', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 1_000_000, openingDate: '2026-10-01' },
  })
  dompet = await ipc.createAccount('cash', { name: 'Dompet', groupId: null, includeInNetWorth: true, creditLimit: null,
    statementDay: null, icon: null, color: null, openingBalance: 0, openingDate: '2026-10-01' })
  const cats = Object.fromEntries((await ipc.listCategories()).map((c) => [c.name, c.id]))
  transferId = await ipc.createTxn({ kind: 'transfer', date: '2026-10-03', accountId: bca, toAccountId: dompet,
    amount: 200_000, splits: [], fee: null, payee: null, note: null, tags: [] })
  expenseId = await ipc.createTxn({ kind: 'expense', date: '2026-10-04', accountId: bca, toAccountId: null,
    amount: 25_000, splits: [{ categoryId: cats['Kopi & jajan']!, amount: 25_000, memo: null }], fee: null,
    payee: 'Kopi Kenangan', note: null, tags: ['ngopi'] })
})

async function settle() {
  await flushPromises()
  await nextTick()
  await flushPromises()
}

async function mountList() {
  const w = mount(TxnList, { props: { virtual: false }, global: { plugins: [i18n] }, attachTo: document.body })
  await settle()
  return w
}

const rowEl = (id: string) => document.body.querySelector<HTMLElement>(`[data-txn-id="${id}"]`)!

describe('TxnList', () => {
  beforeEach(async () => {
    document.body.innerHTML = ''
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
    await useTransactionsStore().reload()
  })

  it('transfer tampil A → B dengan nominal netral', async () => {
    await mountList()
    const el = rowEl(transferId)
    expect(el.textContent).toContain('BCA')
    expect(el.textContent).toContain('Dompet')
    expect(el.querySelector('[data-testid="txn-accounts"]')!.textContent!.replace(/\s+/g, ' ').trim()).toBe('BCA → Dompet')
    const amount = el.querySelector('[data-testid="txn-amount"]')!
    expect(amount.className).not.toContain('text-expense')
    expect(amount.className).not.toContain('text-income')
    expect(rowEl(expenseId).querySelector('[data-testid="txn-amount"]')!.className).toContain('text-expense')
  })

  it('kolom saldo berjalan hanya saat filter akun aktif', async () => {
    await mountList()
    expect(document.body.querySelector('[data-testid="txn-running"]')).toBeNull()
    await useTransactionsStore().setQuery({ accountId: bca })
    await settle()
    const running = rowEl(expenseId).querySelector('[data-testid="txn-running"]')
    expect(running).not.toBeNull()
    // 1.000.000 − 200.000 − 25.000
    expect(running!.textContent).toContain('775.000')
  })

  it('Delete pada baris fokus memanggil store.remove', async () => {
    await mountList()
    const store = useTransactionsStore()
    const spy = vi.spyOn(store, 'remove').mockResolvedValue()
    const el = rowEl(expenseId)
    el.focus()
    el.dispatchEvent(new KeyboardEvent('keydown', { key: 'Delete', bubbles: true, cancelable: true }))
    await settle()
    expect(spy).toHaveBeenCalledWith(expenseId)
  })

  it('Enter membuka editor mode edit', async () => {
    await mountList()
    const el = rowEl(expenseId)
    el.focus()
    el.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }))
    await settle()
    expect(useUiStore().txnEditor).toEqual({ mode: 'edit', id: expenseId })
  })

  it('Ctrl+D membuka editor mode duplikasi', async () => {
    await mountList()
    rowEl(expenseId).dispatchEvent(new KeyboardEvent('keydown', { key: 'd', ctrlKey: true, bubbles: true, cancelable: true }))
    await settle()
    expect(useUiStore().txnEditor).toEqual({ mode: 'duplicate', id: expenseId })
  })

  it('panah bawah memindah fokus ke baris berikutnya', async () => {
    await mountList()
    const rows = [...document.body.querySelectorAll<HTMLElement>('[data-txn-id]')]
    rows[0]!.focus()
    rows[0]!.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true }))
    await settle()
    expect(document.activeElement).toBe(rows[1])
  })

  it('baris opening: item menu Edit nonaktif', async () => {
    await mountList()
    const opening = useTransactionsStore().rows.find((r) => r.kind === 'opening' && r.accountId === bca)!
    const trigger = rowEl(opening.id).querySelector<HTMLElement>('[data-testid="txn-menu"]')!
    trigger.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }))
    await settle()
    expect(rowEl(opening.id).querySelector('svg')!.getAttribute('aria-label')).toBe(i18n.global.t('txn.kinds.opening'))
    expect(i18n.global.te('txn.kinds.opening', 'id') && i18n.global.te('txn.kinds.adjustment', 'en') && i18n.global.te('txn.kinds.valuation', 'id')).toBe(true)
    const edit = document.body.querySelector('[data-testid="txn-menu-edit"]')!
    expect(edit.hasAttribute('data-disabled')).toBe(true)
    expect(document.body.querySelector('[data-testid="txn-menu-delete"]')!.hasAttribute('data-disabled')).toBe(false)
  })

  it('kosong karena filter: pesan filter dan tombol hapus filter', async () => {
    const store = useTransactionsStore()
    await store.setQuery({ search: 'tidak-ada-yang-cocok' })
    await mountList()
    expect(document.body.textContent).toContain(i18n.global.t('txn.emptyFiltered'))
    document.body.querySelector<HTMLElement>('[data-testid="txn-empty-clear"]')!.click()
    await settle()
    expect(store.query.search).toBeNull()
  })

  it('kosong tanpa filter: ajakan mencatat transaksi pertama', async () => {
    useTransactionsStore().rows = []
    await mountList()
    expect(document.body.textContent).toContain(i18n.global.t('txn.empty'))
    document.body.querySelector<HTMLElement>('[data-testid="txn-empty-new"]')!.click()
    expect(useUiStore().txnEditor).toEqual({ mode: 'create' })
  })

  it('baris: tile ikon kategori, kategori anak dengan induk di tooltip, tag sebagai teks', async () => {
    await mountList()
    const el = rowEl(expenseId)
    expect(el.querySelector('[data-testid="txn-icon"]')).not.toBeNull()
    const cat = el.querySelector<HTMLElement>('[data-testid="txn-category"]')!
    expect(cat.textContent!.trim()).toBe('Kopi & jajan')
    expect(cat.title).toBe('Makan & Minum › Kopi & jajan')
    expect(el.querySelector('[data-testid="txn-tag"]')!.textContent).toBe('#ngopi')
    expect(rowEl(transferId).querySelector('[data-testid="txn-icon"]')!.className).toContain('bg-muted')
  })
})
