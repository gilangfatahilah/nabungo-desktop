import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import * as ipc from '@/ipc'
import { useReportsStore } from './reports'

beforeAll(async () => {
  const bca = await ipc.completeOnboarding({
    locale: 'id', periodStartDay: 1, accountKind: 'bank',
    account: { name: 'BCA', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 1_000_000, openingDate: '2026-09-01', contact: null },
  })
  const cats = await ipc.listCategories()
  const gaji = cats.find((c) => c.name === 'Gaji')!.id
  await ipc.createTxn({
    kind: 'income', date: '2026-09-05', accountId: bca, toAccountId: null, amount: 500_000,
    splits: [{ categoryId: gaji, amount: 500_000, memo: null }], fee: null, payee: null, note: null, tags: [],
  })
})

describe('reports store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date(2026, 9, 17))
  })
  afterEach(() => {
    vi.useRealTimers()
    localStorage.clear()
  })

  it('Dasbor memuat 12 periode berakhir di periode saat ini', async () => {
    const s = useReportsStore()
    await s.load()
    expect(s.cashflow).toHaveLength(12)
    expect(s.netWorth).toHaveLength(12)
    expect(s.cashflow.at(-1)!.period).toBe('2026-10')
  })

  it('Laporan memuat rentang + pembanding dan memfokus periode berjalan', async () => {
    const s = useReportsStore()
    await s.loadReport()
    expect(s.range).toBe(12)
    expect(s.rangeCashflow).toHaveLength(13)
    expect(s.span).toHaveLength(12)
    expect(s.rangeNetWorth).toHaveLength(12)
    expect(s.period).toBe('2026-10')
    expect(s.previous!.period).toBe('2026-09')
    expect(s.canNext).toBe(false)
    expect(s.comparison).toEqual([])
    expect(s.trend).toEqual([])
  })

  it('geser fokus dan jenis memuat banding + tren kategori', async () => {
    const s = useReportsStore()
    await s.loadReport()
    await s.shift(-1)
    await s.setKind('income')
    expect(s.period).toBe('2026-09')
    expect(s.canNext).toBe(true)
    expect(s.comparison.map((c) => [c.amount, c.previous])).toEqual([[500_000, 0]])
    expect(s.trendCategoryId).toBe(s.comparison[0]!.categoryId)
    expect(s.trend).toHaveLength(12)
    expect(s.trend.at(-2)!.amount).toBe(500_000)
    await s.shift(1)
    await s.shift(1)
    expect(s.period).toBe('2026-10')
  })

  it('rentang tersimpan dan fokus di luar jendela kembali ke periode berjalan', async () => {
    const s = useReportsStore()
    await s.loadReport()
    await s.select('2026-02')
    await s.setRange(3)
    expect(s.period).toBe('2026-10')
    expect(s.span.map((p) => p.period)).toEqual(['2026-08', '2026-09', '2026-10'])
    expect(localStorage.getItem('nabungo.reports.range')).toBe('3')
    expect(s.canPrev).toBe(true)
    await s.select('2026-08')
    expect(s.canPrev).toBe(false)
  })
})
