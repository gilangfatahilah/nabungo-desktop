import { afterEach, describe, expect, it, vi } from 'vitest'
import type { BudgetOverviewDto, TxnFormDto } from './bindings'
import { __enterRecovery, __leaveRecovery, browserLedger as L } from './browser-ledger'
import { browserCommands } from './browser'

async function data<T>(p: Promise<{ status: 'ok'; data: T } | { status: 'error'; error: unknown }>): Promise<T> {
  const r = await p
  if (r.status !== 'ok') throw new Error(JSON.stringify(r.error))
  return r.data
}

async function account(name: string, opening = 0): Promise<string> {
  return data(L.createAccount('bank', {
    name, groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
    icon: null, color: null, openingBalance: opening, openingDate: '2026-10-01', contact: null,
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
  amountMin: null, amountMax: null, search: null, sort: null, limit: 100, offset: 0 }

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

  it('sortir nominal bertanda & payee NOCASE (kosong di akhir), paginasi stabil', async () => {
    const a = await account('Sort', 1_000_000)
    const c = await category('expense', 'Kopi Sort')
    const kopi = (await data(L.createTxn(form({ accountId: a, date: '2026-10-02', amount: 25_000, payee: 'Kopi Kenangan', splits: [{ categoryId: c, amount: 25_000, memo: null }] })))).id
    const makan = (await data(L.createTxn(form({ accountId: a, date: '2026-10-04', amount: 50_000, note: 'makan 50% diskon', splits: [{ categoryId: c, amount: 50_000, memo: null }] })))).id
    const opening = (await data(L.listTxns({ ...q, accountId: a }))).rows.find((r) => r.kind === 'opening')!.id
    const ids = async (field: 'date' | 'amount' | 'payee', desc: boolean, extra = {}) =>
      (await data(L.listTxns({ ...q, accountId: a, sort: { field, desc }, ...extra }))).rows.map((r) => r.id)
    expect(await ids('amount', false)).toEqual([makan, kopi, opening])
    expect(await ids('amount', true)).toEqual([opening, kopi, makan])
    expect(await ids('payee', false)).toEqual([kopi, makan, opening])
    expect(await ids('payee', true)).toEqual([makan, kopi, opening])
    expect(await ids('date', false)).toEqual([opening, kopi, makan])
    expect([...await ids('amount', true, { limit: 2 }), ...await ids('amount', true, { limit: 2, offset: 2 })]).toEqual([opening, kopi, makan])
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
    const { id } = await data(L.createTxn(form({ accountId: a, amount: 10_000, tags: ['#kantor', 'KANTOR'], splits: [{ categoryId: c, amount: 10_000, memo: null }] })))
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
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date(2030, 0, 1))
    await data(L.setPeriodStartDay(1))
    const a = await account('BT1', 1_000_000)
    const b = await account('BT2')
    const c = await category('expense', 'Belanja BT')
    await data(L.setBudget(c, '2026-10', 100_000))
    await spend(a, c, 25_000, '2026-10-02')
    const before = await data(L.budgetOverview('2026-10'))
    await data(L.createTxn(form({ kind: 'transfer', accountId: a, toAccountId: b, amount: 200_000 })))
    expect(await data(L.budgetOverview('2026-10'))).toEqual(before)
    expect(row(before, c)).toEqual({
      categoryId: c, parentId: null, budget: 100_000, rollover: false, carry: 0, available: 100_000,
      spent: 25_000, remaining: 75_000, level: 'ok', percent: 25, perDay: null,
    })
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
    expect(o.total).toEqual({ budget: 100_000, carry: 0, available: 100_000, spent: 25_000, remaining: 75_000, perDay: null })
  })

  afterEach(() => vi.useRealTimers())

  async function budget(c: string, period: string, amount: number, rollover = false) {
    await data(L.setBudget(c, period, amount))
    if (rollover) await data(L.setBudgetRollover(c, period, true))
  }

  it('carry berantai', async () => {
    const a = await account('CR1', 10_000_000)
    const c = await category('expense', 'Rantai CR')
    await budget(c, '2029-01', 100_000, true)
    await budget(c, '2029-02', 100_000, true)
    await budget(c, '2029-03', 100_000)
    await spend(a, c, 30_000, '2029-01-05')
    await spend(a, c, 50_000, '2029-02-05')
    await spend(a, c, 10_000, '2029-03-05')
    const o = await data(L.budgetOverview('2029-03'))
    expect(row(o, c)).toMatchObject({ rollover: false, carry: 120_000, available: 220_000, remaining: 210_000, level: 'ok', percent: 4 })
    expect(row(await data(L.budgetOverview('2029-02')), c)).toMatchObject({ rollover: true, carry: 70_000, available: 170_000 })
    expect(o.total).toMatchObject({ carry: 120_000, available: 220_000 })
  })

  it('carry reset setelah periode tanpa budget', async () => {
    const c = await category('expense', 'Reset CR')
    await budget(c, '2029-05', 100_000, true)
    await budget(c, '2029-07', 100_000, true)
    expect(row(await data(L.budgetOverview('2029-07')), c)).toMatchObject({ carry: 0, available: 100_000 })
    await budget(c, '2029-06', 100_000)
    expect(row(await data(L.budgetOverview('2029-07')), c)).toMatchObject({ carry: 0 })
  })

  it('carry negatif → exceeded tanpa percent', async () => {
    const a = await account('CR2', 10_000_000)
    const c = await category('expense', 'Negatif CR')
    await budget(c, '2029-09', 50_000, true)
    await spend(a, c, 100_000, '2029-09-10')
    await budget(c, '2029-10', 30_000)
    expect(row(await data(L.budgetOverview('2029-10')), c)).toMatchObject({
      carry: -50_000, available: -20_000, spent: 0, remaining: -20_000, level: 'exceeded', percent: null,
    })
  })

  it('per hari hanya periode berjalan', async () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date(2026, 9, 22, 10))
    const a = await account('PD', 1_000_000)
    const c = await category('expense', 'Harian PD')
    await budget(c, '2026-10', 310_000)
    await spend(a, c, 10_000, '2026-10-05')
    const o = await data(L.budgetOverview('2026-10'))
    expect(o.daysLeft).toBe(10)
    expect(row(o, c)?.perDay).toBe(30_000)
    expect(o.total.perDay).not.toBeNull()
    const next = await data(L.budgetOverview('2026-11'))
    expect(next.daysLeft).toBeNull()
    expect(next.total.perDay).toBeNull()
  })

  it('setBudgetRollover tanpa budget → not_found', async () => {
    const c = await category('expense', 'Flag SR')
    const inc = await category('income', 'Masuk SR')
    expect(await L.setBudgetRollover(c, '2029-01', true)).toMatchObject({ status: 'error', error: { code: 'not_found', entity: 'budget' } })
    expect(await L.setBudgetRollover('x', '2029-01', true)).toMatchObject({ status: 'error', error: { code: 'not_found', entity: 'category' } })
    expect(await L.setBudgetRollover(inc, '2029-01', true)).toMatchObject({ status: 'error', error: { code: 'budget_income_category' } })
    expect(await L.setBudgetRollover(c, '2029-13', true)).toMatchObject({ status: 'error', error: { code: 'invalid_period' } })
    await budget(c, '2029-01', 1)
    await data(L.archiveCategory(c))
    expect(await L.setBudgetRollover(c, '2029-01', true)).toMatchObject({ status: 'error', error: { code: 'archived', entity: 'category' } })
    await data(L.setBudgetRollover(c, '2029-01', false))
  })

  it('salin budget ikut flag rollover', async () => {
    const c = await category('expense', 'Salin SR')
    await budget(c, '2030-01', 40_000, true)
    expect(await data(L.copyBudgets('2030-02'))).toBe(1)
    expect(row(await data(L.budgetOverview('2030-02')), c)).toMatchObject({ budget: 40_000, rollover: true })
    await data(L.setBudget(c, '2030-02', 50_000))
    expect(row(await data(L.budgetOverview('2030-02')), c)).toMatchObject({ budget: 50_000, rollover: true })
  })

  it('alert sekali saat melewati 80% lalu 100%', async () => {
    const a = await account('AL', 10_000_000)
    const c = await category('expense', 'Alert AL')
    const ch = await category('expense', 'Anak AL', c)
    await budget(c, '2030-05', 100_000)
    const alerts = async (amount: number, cat = c) => (await spend(a, cat, amount, '2030-05-10')).alerts
    expect(await alerts(70_000)).toEqual([])
    expect(await alerts(15_000, ch)).toEqual([{ categoryId: c, period: '2030-05', level: 'warning', percent: 85 }])
    expect(await alerts(5_000)).toEqual([])
    expect(await alerts(20_000)).toEqual([{ categoryId: c, period: '2030-05', level: 'exceeded', percent: 110 }])
    expect(await alerts(1_000)).toEqual([])
  })

  it('edit pindah periode → alert periode baru saja', async () => {
    const a = await account('ED', 10_000_000)
    const c = await category('expense', 'Edit ED')
    await budget(c, '2030-07', 100_000)
    await budget(c, '2030-08', 100_000)
    const saved = await spend(a, c, 90_000, '2030-07-10')
    expect(saved.alerts.map((x) => x.period)).toEqual(['2030-07'])
    const line = (amount: number) => [{ categoryId: c, amount, memo: null }]
    expect(await data(L.updateTxn(saved.id, form({ accountId: a, date: '2030-08-10', amount: 90_000, splits: line(90_000) }))))
      .toEqual([{ categoryId: c, period: '2030-08', level: 'warning', percent: 90 }])
    expect(await data(L.updateTxn(saved.id, form({ accountId: a, date: '2030-08-11', amount: 95_000, splits: line(95_000) })))).toEqual([])
    expect(await L.updateTxn('x', form({ accountId: a }))).toMatchObject({ status: 'error', error: { code: 'not_found' } })
  })

  it('transfer tidak memberi alert', async () => {
    const a = await account('TA1', 10_000_000)
    const b = await account('TA2')
    const c = await category('expense', 'Tanpa TA')
    await budget(c, '2026-10', 1)
    const r = await data(L.createTxn(form({ kind: 'transfer', accountId: a, toAccountId: b, amount: 500_000 })))
    expect(r.alerts).toEqual([])
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
      icon: null, color: null, openingBalance: opening, openingDate: '2026-10-01', contact: null,
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
    data(L.createBucket({ name, parentId, targetBp, targetAmount: null, color: null }))
  const err = (code: string) => ({ status: 'error', error: { code } })
  const investment = (name: string, opening: number) => data(L.createAccount('investment', {
    name, groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
    icon: null, color: null, openingBalance: opening, openingDate: '2026-10-01', contact: null,
  }))

  it('nested target is relative', async () => {
    await resetBuckets()
    const aset = await bucket('Aset', null, 7000)
    const saham = await bucket('Saham', aset, 6000)
    const o = await data(L.listAllocation())
    expect(o.buckets.map((b) => b.id)).toEqual([aset, saham])
    expect(o.actuals.find((a) => a.bucketId === saham)!.targetBp).toBe(4200)
    expect(o.actuals.find((a) => a.bucketId === aset)!.targetBp).toBe(7000)
    expect(await L.createBucket({ name: 'Liquid', parentId: null, targetBp: 3001, targetAmount: null, color: null }))
      .toMatchObject(err('allocation_targets_exceed'))
    expect(await L.createBucket({ name: ' aset ', parentId: null, targetBp: 0, targetAmount: null, color: null }))
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
      icon: null, color: null, openingBalance: 0, openingDate: '2026-10-01', contact: null,
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
    expect(await L.createBucket({ name: 'Bluechip', parentId: saham, targetBp: 100, targetAmount: null, color: null })).toMatchObject(err('allocation_depth'))
    expect(await L.updateBucket(aset, { name: 'Aset', parentId: liquid, targetBp: 7000, targetAmount: null, color: null })).toMatchObject(err('allocation_depth'))
    expect(await L.updateBucket(liquid, { name: 'Liquid', parentId: liquid, targetBp: 3000, targetAmount: null, color: null })).toMatchObject(err('allocation_depth'))
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
      icon: null, color: null, openingBalance: opening, openingDate, contact: null,
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

    await data(L.restoreBackup({ kind: 'named', fileName: auto!.fileName }, null))
    expect(await balance(a)).toBe(500_000)
    const list = await data(L.listBackups())
    expect(list.filter((b) => b.kind === 'pre_restore')).toHaveLength(1)
    expect(list[0]!.kind).toBe('pre_restore')
    expect([...list].sort((x, y) => y.createdAt.localeCompare(x.createdAt))).toEqual(list)
  })

  it('restore nama tak dikenal ditolak', async () => {
    expect(await L.restoreBackup({ kind: 'named', fileName: 'auto-20260101T000000Z.db' }, null)).toEqual({
      status: 'error', error: { code: 'backup_invalid', entity: null },
    })
  })

  it('tanpa file di browser: restore dari file batal, laporan startup bersih', async () => {
    expect(await data(L.pickBackupFile())).toBeNull()
    expect(await L.startupReport()).toEqual({ uncleanShutdown: false, integrity: 'skipped' })
  })
})

describe('browser ledger piutang/hutang & rekonsiliasi (paritas Rust)', () => {
  const receivable = (name: string, opening: number, contact: string | null = null) =>
    data(L.createAccount('receivable', {
      name, groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: opening, openingDate: '2026-10-01', contact,
    }))
  const spendOn = (accountId: string, c: string, date: string, amount: number) =>
    data(L.createTxn(form({ accountId, date, amount, splits: [{ categoryId: c, amount, memo: null }] })))

  it('hutang dihitung sebagai liabilitas', async () => {
    const before = await data(L.accountsOverview())
    const [nwBefore] = await data(L.netWorthHistory('2026-10', 1))
    const allocBefore = await data(L.listAllocation())
    await receivable('Hutang ke Andi', -50_000)
    const after = await data(L.accountsOverview())
    expect(after.netWorth.liabilities).toBe(before.netWorth.liabilities + 50_000)
    expect(after.netWorth.assets).toBe(before.netWorth.assets)
    const [nwAfter] = await data(L.netWorthHistory('2026-10', 1))
    expect(nwAfter!.liabilities).toBe(nwBefore!.liabilities + 50_000)
    expect(nwAfter!.assets).toBe(nwBefore!.assets)
    const allocAfter = await data(L.listAllocation())
    expect(allocAfter.gross).toBe(allocBefore.gross)
  })

  it('ubah akun tanpa ganti saldo awal mempertahankan tanda reconciled', async () => {
    const fields = (name: string, openingBalance: number) => ({
      name, groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance, openingDate: '2026-10-01', contact: null,
    })
    const throughOf = async (i: string) => (await data(L.accountsOverview())).accounts.find((a) => a.id === i)!.reconciledThrough
    const laci = await data(L.createAccount('cash', fields('Laci', 50_000)))
    await data(L.reconcileAccount(laci, '2026-10-10', 50_000))
    expect(await throughOf(laci)).toBe('2026-10-01')
    await data(L.updateAccount(laci, { ...fields('Laci meja', 50_000), icon: 'wallet' }))
    expect(await throughOf(laci)).toBe('2026-10-01')
    await data(L.updateAccount(laci, fields('Laci meja', 60_000)))
    expect(await throughOf(laci)).toBeNull()
  })

  it('saldo per tanggal tidak menghitung transaksi sesudahnya', async () => {
    const c = (await data(L.listCategories())).find((x) => x.kind === 'expense')!.id
    const laci = await data(L.createAccount('cash', {
      name: 'Laci Saldo', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 50_000, openingDate: '2026-10-01', contact: null,
    }))
    await spendOn(laci, c, '2026-10-20', 10_000)
    expect(await data(L.accountBalanceAsOf(laci, '2026-10-10'))).toBe(50_000)
    expect(await data(L.accountBalanceAsOf(laci, '2026-10-20'))).toBe(40_000)
    expect(await L.accountBalanceAsOf('nope', '2026-10-20')).toMatchObject({ status: 'error', error: { code: 'not_found' } })
  })

  it('kontak dipakai ulang tanpa beda huruf', async () => {
    const a = await receivable('Piutang Budi 1', 10_000, 'Budi')
    const b = await receivable('Piutang Budi 2', 10_000, ' budi')
    const bank = await data(L.createAccount('bank', {
      name: 'Bank Kontak', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 0, openingDate: '2026-10-01', contact: 'Budi',
    }))
    expect((await data(L.listContacts())).filter((c) => c.name.toLowerCase() === 'budi').map((c) => c.name)).toEqual(['Budi'])
    const o = await data(L.accountsOverview())
    const by = (i: string) => o.accounts.find((x) => x.id === i)!
    expect([by(a).contact, by(b).contact, by(bank).contact]).toEqual(['Budi', 'Budi', null])
  })

  it('rekonsiliasi membuat penyesuaian dan menandai', async () => {
    const a = await account('Bank Rek', 100_000)
    const c = await category('expense', 'Belanja Rek')
    const early = await spendOn(a, c, '2026-10-05', 10_000)
    const late = await spendOn(a, c, '2026-10-20', 5_000)
    const r = await data(L.reconcileAccount(a, '2026-10-10', 75_000))
    expect(r.difference).toBe(-15_000)
    expect(r.adjustmentId).not.toBeNull()
    const rows = (await data(L.listTxns({ ...q, accountId: a }))).rows
    const flag = (i: string) => rows.find((x) => x.id === i)!.reconciled
    expect(rows.find((x) => x.id === r.adjustmentId)!.kind).toBe('adjustment')
    expect([flag(early.id), flag(r.adjustmentId!), flag(late.id)]).toEqual([true, true, false])
    expect(r.marked).toBe(3)
    expect((await data(L.listTxns(q))).rows.every((x) => x.reconciled === null)).toBe(true)
    const o = await data(L.accountsOverview())
    expect(o.accounts.find((x) => x.id === a)!.reconciledThrough).toBe('2026-10-10')
    expect(await balance(a)).toBe(70_000)

    await data(L.updateTxn(early.id, form({ accountId: a, date: '2026-10-05', amount: 11_000, splits: [{ categoryId: c, amount: 11_000, memo: null }] })))
    const edited = (await data(L.listTxns({ ...q, accountId: a }))).rows.find((x) => x.id === early.id)!
    expect(edited.reconciled).toBe(false)
  })

  it('saldo cocok tidak membuat transaksi', async () => {
    const a = await account('Bank Cocok', 40_000)
    const r = await data(L.reconcileAccount(a, '2026-10-10', 40_000))
    expect(r).toEqual({ adjustmentId: null, difference: 0, marked: 1 })
    expect((await data(L.listTxns({ ...q, accountId: a }))).total).toBe(1)
  })

  it('akun diarsip ditolak', async () => {
    const a = await account('Bank Arsip Rek', 1_000)
    await data(L.archiveAccount(a))
    expect(await L.reconcileAccount(a, '2026-10-10', 0)).toMatchObject({ status: 'error', error: { code: 'archived', entity: 'account' } })
    expect(await L.reconcileAccount('x', '2026-10-10', 0)).toMatchObject({ status: 'error', error: { code: 'not_found', entity: 'account' } })
    expect(await L.reconcileAccount(a, '2026-10-10', 2_000_000_000_000)).toMatchObject({ status: 'error', error: { code: 'txn_amount_too_large' } })
  })
})

describe('browser ledger tag & payee (CAT-04, paritas Rust)', () => {
  const tagCount = async (name: string) => (await data(L.listTagUsage())).find((t) => t.name === name)?.txnCount

  it('tag tak terpakai tetap terdaftar dan jumlah hanya transaksi aktif', async () => {
    const a = await account('TG1', 100_000)
    const c = await category('expense', 'Makan TG1')
    await data(L.createTag('kosong-tg1'))
    const t = await data(L.createTxn(form({ accountId: a, amount: 1_000, splits: [{ categoryId: c, amount: 1_000, memo: null }], tags: ['dipakai-tg1'] })))
    await data(L.createTxn(form({ accountId: a, amount: 1_000, splits: [{ categoryId: c, amount: 1_000, memo: null }], tags: ['dipakai-tg1'] })))
    await data(L.deleteTxn(t.id))
    expect(await tagCount('kosong-tg1')).toBe(0)
    expect(await tagCount('dipakai-tg1')).toBe(1)
    expect(await data(L.listTags())).toContain('kosong-tg1')
  })

  it('ganti nama tag beda huruf saja berhasil; nama tag lain → duplicate', async () => {
    const k = await data(L.createTag('kantor-tg2'))
    await data(L.createTag('rumah-tg2'))
    expect(await L.createTag('#Kantor-TG2')).toEqual({ status: 'error', error: { code: 'duplicate', entity: 'tag' } })
    await data(L.renameTag(k, 'Kantor-tg2'))
    expect(await tagCount('Kantor-tg2')).toBe(0)
    expect(await L.renameTag(k, 'RUMAH-tg2')).toEqual({ status: 'error', error: { code: 'duplicate', entity: 'tag' } })
    expect(await L.renameTag(k, ' # ')).toEqual({ status: 'error', error: { code: 'name_empty', entity: null } })
  })

  it('gabung tag memindah transaksi; gabung ke diri sendiri → merge_into_self', async () => {
    const a = await account('TG3', 100_000)
    const c = await category('expense', 'Makan TG3')
    const t = await data(L.createTxn(form({ accountId: a, amount: 1_000, splits: [{ categoryId: c, amount: 1_000, memo: null }], tags: ['a-tg3'] })))
    const usage = await data(L.listTagUsage())
    const src = usage.find((x) => x.name === 'a-tg3')!.id
    const dst = await data(L.createTag('b-tg3'))
    expect(await L.mergeTag(src, src)).toEqual({ status: 'error', error: { code: 'merge_into_self', entity: null } })
    await data(L.mergeTag(src, dst))
    expect((await data(L.getTxn(t.id))).tags).toEqual(['b-tg3'])
    expect(await tagCount('a-tg3')).toBeUndefined()
    expect(await L.deleteTag(src)).toEqual({ status: 'error', error: { code: 'not_found', entity: 'tag' } })
  })

  it('hapus payee mengosongkan payee transaksi termasuk yang terhapus', async () => {
    const a = await account('PY1', 100_000)
    const c = await category('expense', 'Makan PY1')
    const t = await data(L.createTxn(form({ accountId: a, amount: 1_000, splits: [{ categoryId: c, amount: 1_000, memo: null }], payee: 'Warung PY1' })))
    const p = (await data(L.listPayeeUsage())).find((x) => x.name === 'Warung PY1')!
    expect(p.txnCount).toBe(1)
    expect(p.lastDate).toBe('2026-10-02')
    await data(L.deleteTxn(t.id))
    await data(L.deletePayee(p.id))
    await data(L.undoDeleteTxn(t.id))
    expect((await data(L.getTxn(t.id))).payee).toBeNull()
    expect((await data(L.listPayeeUsage())).some((x) => x.id === p.id)).toBe(false)
  })

  it('kategori default payee diarsip → archived; default dipakai saran', async () => {
    const c = await category('expense', 'Belanja PY2')
    const old = await category('expense', 'Lama PY2')
    await data(L.archiveCategory(old))
    expect(await L.createPayee('Toko PY2', old)).toEqual({ status: 'error', error: { code: 'archived', entity: 'category' } })
    expect(await L.createPayee('Toko PY2', 'tidak-ada')).toEqual({ status: 'error', error: { code: 'not_found', entity: 'category' } })
    const p = await data(L.createPayee('Toko PY2', null))
    await data(L.updatePayee(p, 'TOKO PY2', c))
    expect((await data(L.payeeSuggestion('toko py2')))?.categoryId).toBe(c)
    expect(await L.createPayee('toko py2', null)).toEqual({ status: 'error', error: { code: 'duplicate', entity: 'payee' } })
  })

  it('payee dengan kategori default terarsip tetap bisa diubah namanya', async () => {
    const c = await category('expense', 'Jajan PY5')
    const other = await category('expense', 'Lain PY5')
    const p = await data(L.createPayee('warung PY5', c))
    await data(L.archiveCategory(c))
    await data(L.updatePayee(p, 'Warung PY5', c))
    await data(L.updatePayee(p, 'Warung PY5', other))
    expect(await L.updatePayee(p, 'Warung PY5', c)).toEqual({ status: 'error', error: { code: 'archived', entity: 'category' } })
  })

  it('gabung payee memindah transaksi termasuk yang terhapus', async () => {
    const a = await account('PY3', 100_000)
    const c = await category('expense', 'Makan PY3')
    const t = await data(L.createTxn(form({ accountId: a, amount: 1_000, splits: [{ categoryId: c, amount: 1_000, memo: null }], payee: 'Indomart PY3' })))
    await data(L.deleteTxn(t.id))
    const usage = await data(L.listPayeeUsage())
    const src = usage.find((x) => x.name === 'Indomart PY3')!.id
    const dst = await data(L.createPayee('Indomaret PY3', null))
    expect(await L.mergePayee(src, src)).toEqual({ status: 'error', error: { code: 'merge_into_self', entity: null } })
    await data(L.mergePayee(src, dst))
    await data(L.undoDeleteTxn(t.id))
    expect((await data(L.getTxn(t.id))).payee).toBe('Indomaret PY3')
  })
})

describe('browser ledger lampiran simulasi (TRX-08)', () => {
  it('tambah, hitung di daftar, lalu hapus', async () => {
    const a = await account('LP1', 100_000)
    const c = await category('expense', 'Makan LP1')
    const t = await data(L.createTxn(form({ accountId: a, amount: 1_000, splits: [{ categoryId: c, amount: 1_000, memo: null }] })))
    const added = await data(L.addAttachments(t.id))
    expect(added).toEqual([{ id: expect.any(String), fileName: 'struk-contoh.jpg', mime: 'image/jpeg', sizeBytes: 1234, missing: false }])
    expect(await data(L.listAttachments(t.id))).toEqual(added)
    const row = () => data(L.listTxns({ ...q, accountId: a })).then((p) => p.rows.find((r) => r.id === t.id)!)
    expect((await row()).attachmentCount).toBe(1)
    await data(L.openAttachment(added[0]!.id))
    await data(L.removeAttachment(added[0]!.id))
    expect((await row()).attachmentCount).toBe(0)
    expect(await L.removeAttachment(added[0]!.id)).toEqual({ status: 'error', error: { code: 'not_found', entity: 'attachment' } })
    expect(await L.addAttachments('tidak-ada')).toEqual({ status: 'error', error: { code: 'not_found', entity: 'transaction' } })
  })
})

describe('browser ledger transaksi terjadwal (TRX-06, paritas Rust)', () => {
  afterEach(() => vi.useRealTimers())
  const today = () => {
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date(2026, 9, 21, 10))
  }
  const monthly = { freq: 'monthly' as const, interval: 1, until: null }

  async function setup(tag: string) {
    const a = await account(`R ${tag}`, 1_000_000)
    const c = await category('expense', `Listrik ${tag}`)
    const tmpl = (date: string, amount = 10_000) => form({ date, accountId: a, amount, splits: [{ categoryId: c, amount, memo: null }] })
    const rule = async (id: string) => (await data(L.listRules())).find((r) => r.id === id)!
    const count = async () => (await data(L.listTxns({ ...q, accountId: a }))).rows.filter((r) => r.kind === 'expense').length
    return { a, c, tmpl, rule, count }
  }

  it('kejadian lampau jatuh tempo; konfirmasi dua kali → occurrence_stale', async () => {
    today()
    const { tmpl, rule, count } = await setup('R1')
    const id = await data(L.createRule({ form: tmpl('2026-07-25'), recurrence: monthly, autoPost: false }))
    expect((await rule(id)).due).toEqual(['2026-07-25', '2026-08-25', '2026-09-25'])
    await data(L.confirmOccurrence(id, '2026-07-25', null))
    expect(await L.confirmOccurrence(id, '2026-07-25', null)).toEqual({ status: 'error', error: { code: 'occurrence_stale', entity: null } })
    expect(await count()).toBe(1)
    expect((await rule(id)).form.date).toBe('2026-08-25')
  })

  it('lewati dengan tanggal salah → occurrence_stale; lewati tidak membuat transaksi', async () => {
    today()
    const { tmpl, rule, count } = await setup('R2')
    const id = await data(L.createRule({ form: tmpl('2026-09-25'), recurrence: monthly, autoPost: false }))
    expect(await L.skipOccurrence(id, '2026-09-26')).toEqual({ status: 'error', error: { code: 'occurrence_stale', entity: null } })
    await data(L.skipOccurrence(id, '2026-09-25'))
    expect(await count()).toBe(0)
    expect((await rule(id)).due).toEqual([])
  })

  it('validasi interval dan until', async () => {
    const { tmpl } = await setup('R3')
    expect(await L.createRule({ form: tmpl('2026-11-01'), recurrence: { ...monthly, interval: 0 }, autoPost: false }))
      .toEqual({ status: 'error', error: { code: 'recurrence_interval', entity: null } })
    expect(await L.createRule({ form: tmpl('2026-11-01'), recurrence: { ...monthly, until: '2026-10-31' }, autoPost: false }))
      .toEqual({ status: 'error', error: { code: 'recurrence_until', entity: null } })
  })

  it('auto-post dicatat saat daftar dimuat', async () => {
    today()
    const { tmpl, rule, count } = await setup('R4')
    const id = await data(L.createRule({ form: tmpl('2026-09-25'), recurrence: monthly, autoPost: true }))
    expect((await rule(id)).due).toEqual([])
    expect(await count()).toBe(1)
  })

  it('ubah jadwal selesai tidak memunculkan lagi kejadian terakhir', async () => {
    today()
    const { tmpl, rule, count } = await setup('R6')
    const until = { ...monthly, until: '2026-09-25' }
    const id = await data(L.createRule({ form: tmpl('2026-08-25'), recurrence: until, autoPost: false }))
    expect(await data(L.confirmAllDue(id))).toBe(2)
    const r = await rule(id)
    expect(r.ended).toBe(true)
    await data(L.updateRule(id, { form: r.form, recurrence: until, autoPost: true }))
    expect((await rule(id)).ended).toBe(true)
    await data(L.updateRule(id, { form: r.form, recurrence: { ...monthly, until: '2026-12-31' }, autoPost: true }))
    const after = await rule(id)
    expect(after.ended).toBe(false)
    expect(after.form.date).toBe('2026-10-25')
    expect(after.due).toEqual([])
    expect(await count()).toBe(2)
  })

  it('ubah jadwal di tanggal akhir bulan yang ter-clamp mempertahankan tgl 31', async () => {
    today()
    const { tmpl, rule } = await setup('R7')
    const id = await data(L.createRule({ form: tmpl('2026-01-31'), recurrence: monthly, autoPost: false }))
    await data(L.skipOccurrence(id, '2026-01-31'))
    expect((await rule(id)).form.date).toBe('2026-02-28')
    await data(L.updateRule(id, { form: tmpl('2026-02-28', 12_000), recurrence: monthly, autoPost: false }))
    await data(L.skipOccurrence(id, '2026-02-28'))
    expect((await rule(id)).form.date).toBe('2026-03-31')
    await data(L.updateRule(id, { form: tmpl('2026-04-15', 12_000), recurrence: monthly, autoPost: false }))
    await data(L.skipOccurrence(id, '2026-04-15'))
    expect((await rule(id)).form.date).toBe('2026-05-15')
  })

  it('hapus jadwal membiarkan transaksi; konfirmasi semua mengembalikan jumlah', async () => {
    today()
    const { tmpl, count } = await setup('R5')
    const id = await data(L.createRule({ form: tmpl('2026-08-25'), recurrence: monthly, autoPost: false }))
    expect(await data(L.confirmAllDue(id))).toBe(2)
    expect(await data(L.confirmAllDue(id))).toBe(0)
    await data(L.deleteRule(id))
    expect(await count()).toBe(2)
    expect((await data(L.listRules())).some((r) => r.id === id)).toBe(false)
    expect(await L.deleteRule(id)).toEqual({ status: 'error', error: { code: 'not_found', entity: 'recurring_rule' } })
  })
})

describe('browser ledger mode pemulihan (P11)', () => {
  const C = browserCommands
  afterEach(() => __leaveRecovery())

  it('normal: tidak aktif dan command pemulihan ditolak', async () => {
    expect((await C.recoveryStatus()).active).toBe(false)
    expect(await C.recoverFromBackup({ kind: 'named', fileName: 'x.db' }, { kind: 'none' })).toEqual({
      status: 'error', error: { code: 'recovery_unavailable', entity: null },
    })
    expect(await C.recoveryStartFresh()).toEqual({ status: 'error', error: { code: 'recovery_unavailable', entity: null } })
  })

  it('aktif: command ledger → recovery_required, command pemulihan tetap jalan', async () => {
    __enterRecovery('unreadable')
    expect(await C.recoveryStatus()).toMatchObject({ active: true, reason: 'unreadable' })
    expect(await C.getOnboardingStatus()).toEqual({ status: 'error', error: { code: 'recovery_required', entity: null } })
    expect(await C.securityStatus()).toEqual({ status: 'error', error: { code: 'recovery_required', entity: null } })
    expect((await C.listBackups()).status).toBe('ok')
  })

  it('recoverFromBackup mengembalikan snapshot dan keluar dari pemulihan', async () => {
    const a = await account('Pulih A', 300_000)
    const c = await category('expense', 'Belanja pulih')
    await data(L.backupNow())
    const [auto] = await data(L.listBackups())
    await data(L.createTxn(form({ accountId: a, amount: 50_000, splits: [{ categoryId: c, amount: 50_000, memo: null }] })))
    expect(await balance(a)).toBe(250_000)
    __enterRecovery('tooNew')
    expect(await C.recoverFromBackup({ kind: 'named', fileName: 'tidak-ada.db' }, { kind: 'none' })).toEqual({
      status: 'error', error: { code: 'backup_invalid', entity: null },
    })
    expect(await data(C.inspectBackupSource({ kind: 'named', fileName: auto!.fileName }))).toEqual({ needsKey: false })
    await data(C.recoverFromBackup({ kind: 'named', fileName: auto!.fileName }, { kind: 'none' }))
    expect((await C.recoveryStatus()).active).toBe(false)
    expect(await balance(a)).toBe(300_000)
  })

  // Terakhir di file: state browser dikosongkan.
  it('recoveryStartFresh → onboarding belum selesai', async () => {
    __enterRecovery('unreadable')
    await data(C.recoveryStartFresh())
    expect((await C.recoveryStatus()).active).toBe(false)
    expect(await data(C.getOnboardingStatus())).toBe(false)
  })
})
