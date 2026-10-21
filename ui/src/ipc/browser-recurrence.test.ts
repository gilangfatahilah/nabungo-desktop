import { describe, expect, it } from 'vitest'
import type { RecurrenceDto } from './bindings'
import { nextAfter, occurrences } from './browser-recurrence'

const rec = (freq: RecurrenceDto['freq'], interval = 1, until: string | null = null): RecurrenceDto => ({ freq, interval, until })

// Kasus sama dengan test `crates/core/src/recurrence.rs`.
describe('browser recurrence (paritas core)', () => {
  it('bulanan tanggal 31 di-clamp ke akhir bulan tanpa bergeser', () => {
    expect(occurrences(rec('monthly'), '2026-01-31', 4)).toEqual(['2026-01-31', '2026-02-28', '2026-03-31', '2026-04-30'])
  })

  it('tahunan 29 Feb → 28 Feb di tahun non-kabisat', () => {
    expect(occurrences(rec('yearly'), '2028-02-29', 5)).toEqual(['2028-02-29', '2029-02-28', '2030-02-28', '2031-02-28', '2032-02-29'])
  })

  it('interval 2 bulanan', () => {
    expect(occurrences(rec('monthly', 2), '2026-01-31', 3)).toEqual(['2026-01-31', '2026-03-31', '2026-05-31'])
    expect(occurrences(rec('monthly', 1), '2026-11-15', 3)).toEqual(['2026-11-15', '2026-12-15', '2027-01-15'])
  })

  it('harian dan mingguan', () => {
    expect(nextAfter(rec('daily', 3), '2026-12-30', '2026-12-30')).toBe('2027-01-02')
    expect(nextAfter(rec('weekly', 2), '2026-10-19', '2026-10-19')).toBe('2026-11-02')
  })

  it('until inklusif mengakhiri jadwal', () => {
    const r = rec('monthly', 1, '2026-03-31')
    expect(occurrences(r, '2026-01-31', 10)).toEqual(['2026-01-31', '2026-02-28', '2026-03-31'])
    expect(nextAfter(r, '2026-01-31', '2026-03-31')).toBeNull()
    expect(occurrences(rec('daily', 1, '2026-01-01'), '2026-01-01', 5)).toEqual(['2026-01-01'])
  })
})
