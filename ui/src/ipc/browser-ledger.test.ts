import { describe, expect, it } from 'vitest'
import type { TxnFormDto } from './bindings'
import { browserLedger as L } from './browser-ledger'

async function data<T>(p: Promise<{ status: 'ok'; data: T } | { status: 'error'; error: unknown }>): Promise<T> {
  const r = await p
  if (r.status !== 'ok') throw new Error(JSON.stringify(r.error))
  return r.data
}

async function account(name: string, opening = 0): Promise<string> {
  return data(L.createAccount('bank', {
    name, groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
    icon: null, color: null, openingBalance: opening, openingDate: '2026-10-01',
  }))
}

async function category(kind: 'income' | 'expense', name: string, parentId: string | null = null): Promise<string> {
  return data(L.createCategory(kind, { name, parentId, icon: null, color: null }))
}

async function balance(id: string): Promise<number> {
  const o = await data(L.accountsOverview())
  return o.accounts.find((a) => a.id === id)!.balance
}

function form(over: Partial<TxnFormDto>): TxnFormDto {
  return {
    kind: 'expense', date: '2026-10-02', accountId: '', toAccountId: null, amount: 0, splits: [],
    fee: null, payee: null, note: null, tags: [], ...over,
  }
}

const q = { dateFrom: null, dateTo: null, accountId: null, categoryId: null, tag: null, payeeId: null,
  amountMin: null, amountMax: null, search: null, limit: 100, offset: 0 }

describe('browser ledger transaksi (paritas Rust)', () => {
  it('transfer dengan biaya: saldo A 793.500, B 200.000', async () => {
    const a = await account('A', 1_000_000)
    const b = await account('B')
    const fee = await category('expense', 'Biaya bank')
    await data(L.createTxn(form({ kind: 'transfer', accountId: a, toAccountId: b, amount: 200_000, fee: { amount: 6_500, categoryId: fee } })))
    expect(await balance(a)).toBe(793_500)
    expect(await balance(b)).toBe(200_000)
  })

  it('split tidak cocok → txn_split_mismatch, tidak tersimpan', async () => {
    const a = await account('S', 0)
    const c = await category('expense', 'Makan S')
    const r = await L.createTxn(form({ accountId: a, amount: 75_000, splits: [{ categoryId: c, amount: 74_999, memo: null }] }))
    expect(r).toEqual({ status: 'error', error: { code: 'txn_split_mismatch', entity: null } })
    expect((await data(L.listTxns({ ...q, accountId: a }))).total).toBe(0)
  })

  it('biaya transfer tanpa kategori → txn_missing_category, tidak tersimpan', async () => {
    const a = await account('F1')
    const b = await account('F2')
    const before = (await data(L.listTxns({ ...q, accountId: a }))).total
    const r = await L.createTxn(form({ kind: 'transfer', accountId: a, toAccountId: b, amount: 200_000, fee: { amount: 6_500, categoryId: null } }))
    expect(r).toEqual({ status: 'error', error: { code: 'txn_missing_category', entity: null } })
    expect((await data(L.listTxns({ ...q, accountId: a }))).total).toBe(before)
  })

  it('transfer nominal ≤ 0 → txn_not_positive', async () => {
    const a = await account('P1')
    const b = await account('P2')
    const r = await L.createTxn(form({ kind: 'transfer', accountId: a, toAccountId: b, amount: 0 }))
    expect(r).toMatchObject({ status: 'error', error: { code: 'txn_not_positive' } })
  })

  it('filter akun + search: saldo berjalan atas riwayat penuh', async () => {
    const a = await account('R', 1_000_000)
    const kopi = await category('expense', 'Kopi R')
    const gaji = await category('income', 'Gaji R')
    await data(L.createTxn(form({ accountId: a, date: '2026-10-02', amount: 25_000, splits: [{ categoryId: kopi, amount: 25_000, memo: null }] })))
    await data(L.createTxn(form({ kind: 'income', accountId: a, date: '2026-10-02', amount: 5_000_000, splits: [{ categoryId: gaji, amount: 5_000_000, memo: null }] })))
    await data(L.createTxn(form({ accountId: a, date: '2026-10-04', amount: 50_000, note: 'makan 50% diskon', splits: [{ categoryId: kopi, amount: 50_000, memo: null }] })))
    const all = await data(L.listTxns({ ...q, accountId: a }))
    expect(all.total).toBe(4)
    expect(all.rows.map((r) => r.runningBalance)).toEqual([5_925_000, 5_975_000, 975_000, 1_000_000])
    const page = await data(L.listTxns({ ...q, accountId: a, search: 'diskon' }))
    expect(page.rows).toHaveLength(1)
    expect(page.rows[0]).toMatchObject({ runningBalance: 5_925_000, accountDelta: -50_000, amount: -50_000 })
  })

  it('search "50%" literal dan tanpa beda huruf', async () => {
    const a = await account('L')
    const c = await category('expense', 'Makan L')
    await data(L.createTxn(form({ accountId: a, amount: 1, note: 'diskon 50% hari ini', splits: [{ categoryId: c, amount: 1, memo: null }] })))
    await data(L.createTxn(form({ accountId: a, amount: 2, note: 'diskon 500 rupiah', payee: 'Kopi Kenangan', splits: [{ categoryId: c, amount: 2, memo: null }] })))
    const s = async (search: string) => (await data(L.listTxns({ ...q, accountId: a, search }))).rows.length
    expect(await s('50%')).toBe(1)
    expect(await s('50_')).toBe(0)
    expect(await s('KENANGAN')).toBe(1)
  })

  it('update menjaga id; hapus + undo memulihkan saldo', async () => {
    const a = await account('U', 100_000)
    const c = await category('expense', 'Makan U')
    const id = await data(L.createTxn(form({ accountId: a, amount: 10_000, tags: ['#kantor', 'KANTOR'], splits: [{ categoryId: c, amount: 10_000, memo: null }] })))
    expect((await data(L.getTxn(id))).tags).toEqual(['kantor'])
    await data(L.updateTxn(id, form({ accountId: a, amount: 20_000, splits: [{ categoryId: c, amount: 20_000, memo: null }] })))
    expect(await balance(a)).toBe(80_000)
    await data(L.deleteTxn(id))
    expect(await balance(a)).toBe(100_000)
    expect(await L.getTxn(id)).toMatchObject({ status: 'error', error: { code: 'not_found', entity: 'transaction' } })
    await data(L.undoDeleteTxn(id))
    expect(await balance(a)).toBe(80_000)
  })
})
