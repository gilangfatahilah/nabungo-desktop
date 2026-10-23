import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import * as ipc from '@/ipc'
import { useAllocationStore } from './allocation'

describe('allocation store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date(2026, 9, 17))
  })
  afterEach(() => vi.useRealTimers())

  it('load memuat overview dan riwayat 12 periode berakhir di periode hari ini', async () => {
    const s = useAllocationStore()
    await s.load()
    expect(s.overview?.buckets).toEqual([])
    expect(s.history).toHaveLength(12)
    expect(s.history.at(-1)?.period).toBe('2026-10')
    expect(s.history[0]?.period).toBe('2025-11')
  })

  it('aksi tulis memuat ulang dari backend', async () => {
    const acc = await ipc.createAccount('bank', {
      name: 'Store BCA', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 400_000, openingDate: '2026-10-01', contact: null,
    })
    const s = useAllocationStore()
    await s.load()
    const id = await s.createBucket({ name: 'Kas', parentId: null, targetBp: 10_000, targetAmount: null, color: null })
    expect(s.overview?.buckets.map((b) => b.name)).toEqual(['Kas'])
    await s.setAccountAllocation(acc, [{ bucketId: id, shareBp: 10_000 }])
    expect(s.overview?.actuals[0]?.actual).toBe(400_000)
    expect(s.history.at(-1)?.buckets).toEqual([{ bucketId: id, actualBp: 10_000 }])
    await s.updateBucket(id, { name: 'Kas Darurat', parentId: null, targetBp: 5_000, targetAmount: null, color: null })
    expect(s.overview?.warnings).toEqual([{ parentId: null, totalBp: 5_000 }])
    await s.deleteBucket(id)
    expect(s.overview?.buckets).toEqual([])
  })
})
