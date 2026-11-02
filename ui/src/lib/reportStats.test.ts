import { afterEach, describe, expect, it, vi } from 'vitest'
import { average, biggestChanges, loadRange, perDay, saveRange, savingsRate } from './reportStats'

describe('reportStats', () => {
  afterEach(() => {
    vi.restoreAllMocks()
    localStorage.clear()
  })

  it('savingsRate bulat dan null tanpa pemasukan', () => {
    expect(savingsRate(8_000_000, 2_600_000)).toBe(33)
    expect(savingsRate(1_000_000, -250_000)).toBe(-25)
    expect(savingsRate(0, -100_000)).toBeNull()
  })

  it('perDay memakai hari berjalan untuk periode aktif', () => {
    expect(perDay(310_000, '2026-10-01', '2026-10-31', '2026-12-01')).toBe(10_000)
    expect(perDay(100_000, '2026-10-01', '2026-10-31', '2026-10-10')).toBe(10_000)
    expect(perDay(100_000, '2026-09-25', '2026-10-24', '2026-09-25')).toBe(100_000)
  })

  it('average bulat, null untuk kosong', () => {
    expect(average([1, 2])).toBe(2)
    expect(average([100, 200, 400])).toBe(233)
    expect(average([])).toBeNull()
  })

  it('biggestChanges memisah naik dan turun', () => {
    const row = (categoryId: string, amount: number, previous: number) => ({ categoryId, amount, previous, children: [] })
    const r = biggestChanges([row('a', 100, 50), row('b', 0, 300), row('c', 80, 80), row('d', 500, 100), row('e', 10, 20)], 1)
    expect(r.up.map((c) => [c.categoryId, c.delta])).toEqual([['d', 400]])
    expect(r.down.map((c) => [c.categoryId, c.delta])).toEqual([['b', -300]])
  })

  it('rentang tersimpan, nilai asing jatuh ke 12', () => {
    expect(loadRange()).toBe(12)
    saveRange(6)
    expect(loadRange()).toBe(6)
    localStorage.setItem('nabungo.reports.range', '7')
    expect(loadRange()).toBe(12)
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('blocked') })
    expect(loadRange()).toBe(12)
  })
})
