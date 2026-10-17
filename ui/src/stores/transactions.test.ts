import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import * as ipc from '@/ipc'
import { emptyForm } from '@/lib/txnForm'
import { useAccountsStore } from './accounts'
import { useTransactionsStore } from './transactions'

const toastMock = vi.hoisted(() => vi.fn())
vi.mock('vue-sonner', () => ({ toast: Object.assign(toastMock, { error: vi.fn() }) }))

async function seed(n: number): Promise<string> {
  const acc = await ipc.createAccount('cash', {
    name: `Dompet ${crypto.randomUUID()}`, groupId: null, includeInNetWorth: true, creditLimit: null,
    statementDay: null, icon: null, color: null, openingBalance: 0, openingDate: '2026-10-01',
  })
  const cat = await ipc.createCategory('expense', { name: `Jajan ${crypto.randomUUID()}`, parentId: null, icon: null, color: null })
  for (let i = 0; i < n; i++) {
    await ipc.createTxn({
      kind: 'expense', date: '2026-10-02', accountId: acc, toAccountId: null, amount: i + 1,
      splits: [{ categoryId: cat, amount: i + 1, memo: null }], fee: null, payee: null, note: null, tags: [],
    })
  }
  return acc
}

describe('transactions store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    toastMock.mockClear()
  })

  it('loadMore menambah halaman kedua tanpa dobel', async () => {
    const acc = await seed(130)
    const s = useTransactionsStore()
    await s.setQuery({ accountId: acc })
    expect(s.rows).toHaveLength(100)
    expect(s.total).toBe(130)
    await s.loadMore()
    expect(s.rows).toHaveLength(130)
    expect(new Set(s.rows.map((r) => r.id)).size).toBe(130)
    await s.loadMore()
    expect(s.rows).toHaveLength(130)
  })

  it('hapus lalu undo setelah loadMore: baris kembali tepat sekali', async () => {
    const acc = await seed(130)
    const s = useTransactionsStore()
    const accounts = useAccountsStore()
    const load = vi.spyOn(accounts, 'load')
    await s.setQuery({ accountId: acc })
    await s.loadMore()
    const target = s.rows[120]!.id
    await s.remove(target)
    expect(s.rows.some((r) => r.id === target)).toBe(false)
    expect(s.rows).toHaveLength(129)
    const opts = toastMock.mock.calls[0]![1] as { action: { onClick: () => void } }
    opts.action.onClick()
    await vi.waitFor(() => expect(load).toHaveBeenCalledTimes(2))
    await vi.waitFor(() => expect(s.rows.filter((r) => r.id === target)).toHaveLength(1))
    expect(s.total).toBe(130)
  })

  it('setQuery mereset ke halaman pertama', async () => {
    const acc = await seed(130)
    const s = useTransactionsStore()
    await s.setQuery({ accountId: acc })
    await s.loadMore()
    await s.setQuery({ search: null })
    expect(s.rows).toHaveLength(100)
  })

  it('save membuat transaksi dan memuat ulang', async () => {
    const acc = await seed(0)
    const cat = await ipc.createCategory('expense', { name: `Makan ${crypto.randomUUID()}`, parentId: null, icon: null, color: null })
    const s = useTransactionsStore()
    await s.setQuery({ accountId: acc })
    const id = await s.save(null, { ...emptyForm('expense', { accountId: acc, feeCategoryId: null, today: '2026-10-17' }), amount: 5_000, categoryId: cat })
    expect(s.rows.map((r) => r.id)).toEqual([id])
    await s.save(id, { ...emptyForm('expense', { accountId: acc, feeCategoryId: null, today: '2026-10-17' }), amount: 7_000, categoryId: cat })
    expect(s.rows[0]!.amount).toBe(-7_000)
  })
})
