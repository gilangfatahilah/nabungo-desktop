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

describe('browser ledger alokasi (paritas Rust)', () => {
  // State browser-ledger dibagi antar test; bucket dibersihkan agar Σ target sibling mulai dari 0.
  async function resetBuckets() {
    const o = await data(L.listAllocation())
    for (const b of [...o.buckets].reverse()) await data(L.deleteBucket(b.id))
  }
  const bucket = (name: string, parentId: string | null, targetBp: number) =>
    data(L.createBucket({ name, parentId, targetBp, color: null }))
  const err = (code: string) => ({ status: 'error', error: { code } })
  const investment = (name: string, opening: number) => data(L.createAccount('investment', {
    name, groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
    icon: null, color: null, openingBalance: opening, openingDate: '2026-10-01',
  }))

  it('nested target is relative', async () => {
    await resetBuckets()
    const aset = await bucket('Aset', null, 7000)
    const saham = await bucket('Saham', aset, 6000)
    const o = await data(L.listAllocation())
    expect(o.buckets.map((b) => b.id)).toEqual([aset, saham])
    expect(o.actuals.find((a) => a.bucketId === saham)!.targetBp).toBe(4200)
    expect(o.actuals.find((a) => a.bucketId === aset)!.targetBp).toBe(7000)
    expect(await L.createBucket({ name: 'Liquid', parentId: null, targetBp: 3001, color: null }))
      .toMatchObject(err('allocation_targets_exceed'))
    expect(await L.createBucket({ name: ' aset ', parentId: null, targetBp: 0, color: null }))
      .toMatchObject({ status: 'error', error: { code: 'duplicate', entity: 'bucket' } })
    expect(await L.deleteBucket(aset)).toMatchObject({ status: 'error', error: { code: 'in_use', entity: 'bucket' } })
  })

  it('split mapping and unallocated', async () => {
    await resetBuckets()
    const saham = await bucket('Saham', null, 6000)
    const liquid = await bucket('Liquid', null, 4000)
    const rdn = await investment('RDN A', 10_000_000)
    const part = await account('Bank A', 1_000_000)
    await data(L.setAccountAllocation(rdn, [{ bucketId: saham, shareBp: 9000 }, { bucketId: liquid, shareBp: 1000 }]))
    await data(L.setAccountAllocation(part, [{ bucketId: liquid, shareBp: 2500 }]))
    const o = await data(L.listAllocation())
    expect(o.mappings.filter((m) => m.accountId === rdn)).toHaveLength(2)
    const actual = (id: string) => o.actuals.find((a) => a.bucketId === id)!.actual
    expect([actual(saham), actual(liquid)]).toEqual([9_000_000, 1_250_000])
    expect(o.unallocated.find((u) => u.accountId === part)).toEqual({ accountId: part, remainingBp: 7500, amount: 750_000 })
    expect(o.unallocated.find((u) => u.accountId === rdn)).toBeUndefined()
    await data(L.setAccountAllocation(rdn, []))
    expect((await data(L.listAllocation())).mappings.filter((m) => m.accountId === rdn)).toEqual([])
  })

  it('rejects shares over 100%', async () => {
    await resetBuckets()
    const a = await bucket('A', null, 5000)
    const b = await bucket('B', null, 5000)
    const acc = await account('Bank S', 1_000)
    expect(await L.setAccountAllocation(acc, [{ bucketId: a, shareBp: 6000 }, { bucketId: b, shareBp: 4001 }]))
      .toMatchObject(err('allocation_shares_exceed'))
    expect(await L.setAccountAllocation(acc, [{ bucketId: a, shareBp: 10 }, { bucketId: a, shareBp: 10 }]))
      .toMatchObject(err('allocation_duplicate_bucket'))
    expect(await L.setAccountAllocation(acc, [{ bucketId: a, shareBp: 10001 }])).toMatchObject(err('allocation_invalid_bp'))
    const cc = await data(L.createAccount('credit_card', {
      name: 'Kartu S', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 0, openingDate: '2026-10-01',
    }))
    expect(await L.setAccountAllocation(cc, [{ bucketId: a, shareBp: 100 }])).toMatchObject(err('allocation_liability'))
    await data(L.setAccountAllocation(acc, [{ bucketId: a, shareBp: 100 }]))
    await data(L.deleteAccount(acc))
    expect((await data(L.listAllocation())).mappings.some((m) => m.accountId === acc)).toBe(false)
  })

  it('rejects third level bucket', async () => {
    await resetBuckets()
    const aset = await bucket('Aset', null, 7000)
    const liquid = await bucket('Liquid', null, 3000)
    const saham = await bucket('Saham', aset, 6000)
    expect(await L.createBucket({ name: 'Bluechip', parentId: saham, targetBp: 100, color: null })).toMatchObject(err('allocation_depth'))
    expect(await L.updateBucket(aset, { name: 'Aset', parentId: liquid, targetBp: 7000, color: null })).toMatchObject(err('allocation_depth'))
    expect(await L.updateBucket(liquid, { name: 'Liquid', parentId: liquid, targetBp: 3000, color: null })).toMatchObject(err('allocation_depth'))
  })

  it('history uses balances at each period end', async () => {
    await resetBuckets()
    const kas = await bucket('Kas', null, 10000)
    const before = await data(L.allocationHistory('2026-10', 2))
    expect(before.map((p) => [p.period, p.date])).toEqual([['2026-09', '2026-09-30'], ['2026-10', '2026-10-31']])
    const bank = await account('Bank H', 500_000)
    const kopi = await category('expense', 'Kopi H')
    await data(L.createTxn(form({ accountId: bank, date: '2026-11-05', amount: 100_000, splits: [{ categoryId: kopi, amount: 100_000, memo: null }] })))
    await data(L.setAccountAllocation(bank, [{ bucketId: kas, shareBp: 10000 }]))
    const [sep, oct] = await data(L.allocationHistory('2026-10', 2))
    // Opening 2026-10-01: tidak ada di September; belanja November tidak memengaruhi akhir Oktober.
    expect(sep!.gross).toBe(before[0]!.gross)
    expect(oct!.gross).toBe(before[1]!.gross + 500_000)
    expect(oct!.buckets).toEqual([{ bucketId: kas, actualBp: Math.trunc((500_000 * 10_000) / oct!.gross) }])
    expect(await L.allocationHistory('2026-13', 2)).toMatchObject(err('invalid_period'))
  })
})

describe('browser ledger laporan (paritas Rust)', () => {
  // Periode 2028 dipakai agar transaksi test lain (2026–2027) tidak ikut terhitung.
  const create = (kind: 'bank' | 'credit_card' | 'investment', name: string, opening: number, openingDate: string) =>
    data(L.createAccount(kind, {
      name, groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: opening, openingDate,
    }))
  const line = (categoryId: string, amount: number) => [{ categoryId, amount, memo: null }]

  it('cashflow excludes transfers and valuation', async () => {
    await data(L.setPeriodStartDay(1))
    const bank = await create('bank', 'Bank L', 0, '2028-01-01')
    const dompet = await create('bank', 'Dompet L', 0, '2028-01-01')
    const rdn = await create('investment', 'RDN L', 0, '2028-01-01')
    const [gaji, kopi, fee] = [await category('income', 'Gaji L'), await category('expense', 'Kopi L'), await category('expense', 'Biaya L')]
    await data(L.createTxn(form({ kind: 'income', accountId: bank, date: '2028-05-01', amount: 8_000_000, splits: line(gaji, 8_000_000) })))
    await data(L.createTxn(form({ accountId: bank, date: '2028-05-02', amount: 25_000, splits: line(kopi, 25_000) })))
    await data(L.createTxn(form({ accountId: bank, date: '2028-05-03', amount: -5_000, splits: line(kopi, -5_000) })))
    await data(L.createTxn(form({ kind: 'transfer', accountId: bank, toAccountId: dompet, date: '2028-05-04', amount: 200_000, fee: { amount: 6_500, categoryId: fee } })))
    await data(L.recordValuation(rdn, '2028-05-05', 750_000))
    const r = await data(L.cashflowReport('2028-05', 2))
    expect(r).toEqual([
      { period: '2028-04', from: '2028-04-01', to: '2028-04-30', income: 0, expense: 0, net: 0 },
      { period: '2028-05', from: '2028-05-01', to: '2028-05-31', income: 8_000_000, expense: 26_500, net: 7_973_500 },
    ])
    expect(await data(L.cashflowReport('2028-05', 200))).toHaveLength(24)
    expect(await data(L.cashflowReport('2028-05', 0))).toHaveLength(1)
    expect(await L.cashflowReport('2028-13', 1)).toMatchObject({ status: 'error', error: { code: 'invalid_period' } })
  })

  it('breakdown rolls up to root', async () => {
    const bank = await create('bank', 'Bank B', 0, '2028-01-01')
    const makan = await category('expense', 'Makan B')
    const [resto, kopi] = [await category('expense', 'Resto B', makan), await category('expense', 'Kopi B', makan)]
    const transport = await category('expense', 'Transport B')
    for (const [c, amount] of [[resto, 50_000], [kopi, 25_000], [transport, 60_000]] as const) {
      await data(L.createTxn(form({ accountId: bank, date: '2028-06-03', amount, splits: line(c, amount) })))
    }
    expect(await data(L.categoryBreakdown('2028-06', 'expense'))).toEqual([
      { categoryId: makan, amount: 75_000 },
      { categoryId: transport, amount: 60_000 },
    ])
    expect(await data(L.categoryBreakdown('2028-06', 'income'))).toEqual([])
  })

  it('net worth history per period end', async () => {
    const before = await data(L.netWorthHistory('2028-09', 3))
    expect(before.map((p) => [p.period, p.date])).toEqual([['2028-07', '2028-07-31'], ['2028-08', '2028-08-31'], ['2028-09', '2028-09-30']])
    await create('bank', 'Bank N', 1_000_000, '2028-07-05')
    const cc = await create('credit_card', 'Kartu N', 0, '2028-07-01')
    const rdn = await create('investment', 'RDN N', 0, '2028-07-01')
    const belanja = await category('expense', 'Belanja N')
    await data(L.createTxn(form({ accountId: cc, date: '2028-08-10', amount: 200_000, splits: line(belanja, 200_000) })))
    await data(L.recordValuation(rdn, '2028-09-15', 500_000))
    const after = await data(L.netWorthHistory('2028-09', 3))
    expect(after.map((p, i) => p.net - before[i]!.net)).toEqual([1_000_000, 800_000, 1_300_000])
    expect(after[1]!.liabilities - before[1]!.liabilities).toBe(200_000)
    expect(after.every((p) => p.net === p.assets - p.liabilities)).toBe(true)
  })
})

describe('browser ledger backup', () => {
  it('restore mengembalikan saldo dan membuat pre_restore', async () => {
    const a = await account('Backup A', 500_000)
    const c = await category('expense', 'Belanja backup')
    expect(await data(L.backupNow())).toBe('browser://backup')
    const [auto] = await data(L.listBackups())
    await data(L.createTxn(form({ accountId: a, amount: 100_000, splits: [{ categoryId: c, amount: 100_000, memo: null }] })))
    expect(await balance(a)).toBe(400_000)

    await data(L.restoreBackup(auto!.fileName))
    expect(await balance(a)).toBe(500_000)
    const list = await data(L.listBackups())
    expect(list.filter((b) => b.kind === 'pre_restore')).toHaveLength(1)
    expect(list[0]!.kind).toBe('pre_restore')
    expect([...list].sort((x, y) => y.createdAt.localeCompare(x.createdAt))).toEqual(list)
  })

  it('restore nama tak dikenal ditolak', async () => {
    expect(await L.restoreBackup('auto-20260101T000000Z.db')).toEqual({
      status: 'error', error: { code: 'backup_invalid', entity: null },
    })
  })

  it('tanpa file di browser: restore dari file batal, laporan startup bersih', async () => {
    expect(await data(L.restoreBackupFromFile())).toBe(false)
    expect(await L.startupReport()).toEqual({ uncleanShutdown: false, integrity: 'skipped' })
  })
})
