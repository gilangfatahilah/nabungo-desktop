import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import * as ipc from '@/ipc'
import { emptyForm, type TxnFormState } from '@/lib/txnForm'
import { useScheduledStore } from './scheduled'
import { useTransactionsStore } from './transactions'

let acc = ''
let cat = ''
const monthly = { freq: 'monthly' as const, interval: 1, until: null }

function state(date: string): TxnFormState {
  return { ...emptyForm('expense', { accountId: acc, feeCategoryId: null, today: date }), amount: 50_000, categoryId: cat }
}

beforeAll(async () => {
  acc = await ipc.createAccount('bank', { name: 'Jadwal ST', groupId: null, includeInNetWorth: true, creditLimit: null,
    statementDay: null, icon: null, color: null, openingBalance: 0, openingDate: '2026-01-01', contact: null })
  cat = await ipc.createCategory('expense', { name: 'Listrik ST', parentId: null, icon: null, color: null })
})

describe('scheduled store (fallback browser)', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date(2026, 9, 21, 10))
  })
  afterEach(() => vi.useRealTimers())

  it('dueCount menghitung kejadian jatuh tempo; confirm mencatat satu transaksi', async () => {
    const s = useScheduledStore()
    const txns = useTransactionsStore()
    await s.create(state('2026-08-01'), monthly, false)
    const id = s.rules.find((r) => r.form.accountId === acc && r.due[0] === '2026-08-01')!.id
    expect(s.dueCount).toBeGreaterThanOrEqual(3)
    const before = s.dueCount
    await txns.reload()
    const rows = txns.total
    await s.confirm(id, '2026-08-01')
    expect(s.dueCount).toBe(before - 1)
    expect(txns.total).toBe(rows + 1)
  })

  it('occurrence_stale: memuat ulang lalu menolak', async () => {
    const s = useScheduledStore()
    const id = await s.create(state('2026-10-01'), monthly, false)
    await s.confirm(id, '2026-10-01')
    s.rules = []
    await expect(s.confirm(id, '2026-10-01')).rejects.toMatchObject({ error: { code: 'occurrence_stale' } })
    expect(s.rules.some((r) => r.id === id)).toBe(true)
  })
})
