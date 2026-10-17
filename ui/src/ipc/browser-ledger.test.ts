import { describe, expect, it } from 'vitest'
import type { BudgetOverviewDto, TxnFormDto } from './bindings'
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

describe('browser ledger budget (paritas Rust)', () => {
  async function spend(a: string, c: string, amount: number, date: string) {
    return data(L.createTxn(form({ accountId: a, date, amount, splits: [{ categoryId: c, amount, memo: null }] })))
  }
  const row = (o: BudgetOverviewDto, c: string) => o.rows.find((r) => r.categoryId === c)

  it('transfer tidak memengaruhi budget', async () => {
    await data(L.setPeriodStartDay(1))
    const a = await account('BT1', 1_000_000)
    const b = await account('BT2')
    const c = await category('expense', 'Belanja BT')
    await data(L.setBudget(c, '2026-10', 100_000))
    await spend(a, c, 25_000, '2026-10-02')
    const before = await data(L.budgetOverview('2026-10'))
    await data(L.createTxn(form({ kind: 'transfer', accountId: a, toAccountId: b, amount: 200_000 })))
    expect(await data(L.budgetOverview('2026-10'))).toEqual(before)
    expect(row(before, c)).toEqual({ categoryId: c, parentId: null, budget: 100_000, spent: 25_000, remaining: 75_000 })
  })

  it('periodOf dan rentang dengan tanggal mulai 25', async () => {
    await data(L.setPeriodStartDay(25))
    expect(await data(L.periodOf('2026-10-24'))).toBe('2026-10')
    expect(await data(L.periodOf('2026-10-25'))).toBe('2026-11')
    expect(await data(L.periodOf('2026-12-26'))).toBe('2027-01')
    const o = await data(L.budgetOverview('2026-01'))
    expect(o).toMatchObject({ period: '2026-01', prev: '2025-12', next: '2026-02', from: '2025-12-25', to: '2026-01-24' })
    expect(await L.budgetOverview('2026-13')).toMatchObject({ status: 'error', error: { code: 'invalid_period' } })
    await data(L.setPeriodStartDay(1))
  })

  it('salin budget hanya mengisi yang kosong', async () => {
    const x = await category('expense', 'Salin X')
    const y = await category('expense', 'Salin Y')
    await data(L.setBudget(x, '2027-03', 10))
    await data(L.setBudget(y, '2027-03', 20))
    await data(L.setBudget(x, '2027-04', 5))
    expect(await data(L.copyBudgets('2027-04'))).toBe(1)
    const o = await data(L.budgetOverview('2027-04'))
    expect([row(o, x)?.budget, row(o, y)?.budget]).toEqual([5, 20])
    expect(await data(L.copyBudgets('2027-04'))).toBe(0)
  })

  it('parent mencakup realisasi child; total dari baris teratas yang dibudget', async () => {
    const a = await account('PC', 1_000_000)
    const p = await category('expense', 'Induk PC')
    const ch = await category('expense', 'Anak PC', p)
    await data(L.setBudget(p, '2027-06', 100_000))
    await data(L.setBudget(ch, '2027-06', 40_000))
    await spend(a, ch, 30_000, '2027-06-03')
    await spend(a, ch, -5_000, '2027-06-04')
    const o = await data(L.budgetOverview('2027-06'))
    expect(row(o, p)).toMatchObject({ spent: 25_000, remaining: 75_000 })
    expect(row(o, ch)).toMatchObject({ parentId: p, spent: 25_000 })
    const i = o.rows.findIndex((r) => r.categoryId === p)
    expect(o.rows[i + 1]?.categoryId).toBe(ch)
    expect(o.total).toEqual({ budget: 100_000, spent: 25_000, remaining: 75_000 })
  })

  it('aturan setBudget', async () => {
    const inc = await category('income', 'Pemasukan SB')
    const c = await category('expense', 'Pengeluaran SB')
    expect(await L.setBudget(inc, '2026-10', 1)).toMatchObject({ status: 'error', error: { code: 'budget_income_category' } })
    expect(await L.setBudget(c, '2026-10', -1)).toMatchObject({ status: 'error', error: { code: 'budget_negative' } })
    expect(await L.setBudget('x', '2026-10', 1)).toMatchObject({ status: 'error', error: { code: 'not_found', entity: 'category' } })
    await data(L.archiveCategory(c))
    expect(await L.setBudget(c, '2026-10', 1)).toMatchObject({ status: 'error', error: { code: 'archived', entity: 'category' } })
    await data(L.setBudget(c, '2026-10', null))
  })
})

describe('browser ledger valuation (paritas Rust)', () => {
  async function investment(name: string, opening: number): Promise<string> {
    return data(L.createAccount('investment', {
      name, groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: opening, openingDate: '2026-10-01',
    }))
  }

  it('valuation mencatat selisih terhadap saldo per tanggal', async () => {
    const rdn = await investment('RDN V', 10_000_000)
    const bank = await account('Bank V', 5_000_000)
    await data(L.createTxn(form({ kind: 'transfer', date: '2026-10-20', accountId: bank, toAccountId: rdn, amount: 1_000_000 })))
    await data(L.recordValuation(rdn, '2026-10-10', 9_500_000))
    expect(await balance(rdn)).toBe(10_500_000)
    await data(L.recordValuation(rdn, '2026-10-31', 10_750_000))
    expect(await balance(rdn)).toBe(10_750_000)
    const h = await data(L.valuationHistory(rdn))
    expect(h.map((p) => [p.date, p.change, p.value])).toEqual([
      ['2026-10-31', 250_000, 10_750_000],
      ['2026-10-10', -500_000, 9_500_000],
    ])
  })

  it('aturan valuation', async () => {
    const rdn = await investment('RDN R', 1_000)
    const bank = await account('Bank R', 1_000)
    expect(await L.recordValuation(bank, '2026-10-02', 2_000)).toMatchObject({ status: 'error', error: { code: 'txn_valuation_non_investment' } })
    expect(await L.recordValuation(rdn, '2026-10-02', 1_000)).toMatchObject({ status: 'error', error: { code: 'valuation_unchanged' } })
    expect(await L.recordValuation('x', '2026-10-02', 1)).toMatchObject({ status: 'error', error: { code: 'not_found', entity: 'account' } })
    const v = await data(L.recordValuation(rdn, '2026-10-02', 1_500))
    expect(await L.updateTxn(v, form({ accountId: rdn, amount: 1 }))).toMatchObject({ status: 'error', error: { code: 'txn_not_editable' } })
    await data(L.archiveAccount(rdn))
    expect(await L.recordValuation(rdn, '2026-10-03', 2_000)).toMatchObject({ status: 'error', error: { code: 'archived', entity: 'account' } })
  })
})
