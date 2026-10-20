import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import * as ipc from '@/ipc'
import { useBudgetStore } from './budget'

describe('budget store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date(2026, 9, 17))
  })
  afterEach(() => vi.useRealTimers())

  it('init memakai periode hari ini; go pindah lewat kunci prev/next dari backend', async () => {
    const s = useBudgetStore()
    await s.init()
    expect(s.period).toBe('2026-10')
    await s.go('prev')
    expect(s.overview?.period).toBe('2026-09')
    await s.go('next')
    await s.go('next')
    expect(s.period).toBe('2026-11')
  })

  it('aksi tulis memuat ulang dari backend', async () => {
    const cat = await ipc.createCategory('expense', { name: 'Store B', parentId: null, icon: null, color: null })
    const s = useBudgetStore()
    await s.load('2026-10')
    await s.setAmount(cat, 70_000)
    expect(s.overview?.rows.find((r) => r.categoryId === cat)?.remaining).toBe(70_000)
    await s.setAmount(cat, null)
    expect(s.overview?.rows.find((r) => r.categoryId === cat)?.budget).toBeNull()
    await s.setAmount(cat, 5)
    await s.load('2026-11')
    expect(await s.copyPrevious()).toBe(1)
    expect(s.overview?.rows.find((r) => r.categoryId === cat)?.budget).toBe(5)
  })
})
