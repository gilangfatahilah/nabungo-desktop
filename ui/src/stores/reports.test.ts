import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import * as ipc from '@/ipc'
import { useReportsStore } from './reports'

beforeAll(async () => {
  const bca = await ipc.completeOnboarding({
    locale: 'id', periodStartDay: 1, accountKind: 'bank',
    account: { name: 'BCA', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 1_000_000, openingDate: '2026-09-01' },
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
  afterEach(() => vi.useRealTimers())

  it('memuat 12 periode berakhir di periode saat ini dan memilihnya', async () => {
    const s = useReportsStore()
    await s.load()
    expect(s.cashflow).toHaveLength(12)
    expect(s.netWorth).toHaveLength(12)
    expect(s.cashflow.at(-1)!.period).toBe('2026-10')
    expect(s.period).toBe('2026-10')
    expect(s.breakdown).toEqual([])
  })

  it('memilih periode dan jenis memuat ulang breakdown', async () => {
    const s = useReportsStore()
    await s.load()
    await s.select('2026-09')
    await s.setKind('income')
    expect(s.period).toBe('2026-09')
    expect(s.kind).toBe('income')
    expect(s.breakdown.map((b) => b.amount)).toEqual([500_000])
  })
})
