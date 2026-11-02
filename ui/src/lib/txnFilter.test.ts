import { describe, expect, it } from 'vitest'
import { activeFilterCount, periodOf, periodRange } from './txnFilter'

const EMPTY = { dateFrom: null, dateTo: null, accountId: null, kind: null, categoryId: null, tag: null, payeeId: null,
  amountMin: null, amountMax: null, search: null }
const today = '2026-10-17'

describe('periode filter', () => {
  it('bulan ini: tanggal 1 sampai akhir bulan', () => {
    expect(periodRange('thisMonth', today)).toEqual({ dateFrom: '2026-10-01', dateTo: '2026-10-31' })
    expect(periodRange('thisMonth', '2028-02-10')).toEqual({ dateFrom: '2028-02-01', dateTo: '2028-02-29' })
  })

  it('30 hari: hari ini dan 29 hari sebelumnya, melewati batas bulan', () => {
    expect(periodRange('last30', '2026-03-05')).toEqual({ dateFrom: '2026-02-04', dateTo: '2026-03-05' })
  })

  it('semua waktu mengosongkan rentang', () => {
    expect(periodRange('all', today)).toEqual({ dateFrom: null, dateTo: null })
  })

  it('periodOf mengenali preset dari query, selain itu kustom', () => {
    expect(periodOf(EMPTY, today)).toBe('all')
    expect(periodOf({ ...EMPTY, dateFrom: '2026-10-01', dateTo: '2026-10-31' }, today)).toBe('thisMonth')
    expect(periodOf({ ...EMPTY, dateFrom: '2026-09-18', dateTo: '2026-10-17' }, today)).toBe('last30')
    expect(periodOf({ ...EMPTY, dateFrom: '2026-10-02', dateTo: null }, today)).toBe('custom')
  })
})

describe('activeFilterCount', () => {
  it('menghitung semua filter yang terisi, termasuk cari', () => {
    expect(activeFilterCount(EMPTY)).toBe(0)
    expect(activeFilterCount({ ...EMPTY, search: 'kopi', amountMin: 0, tag: 'x' })).toBe(3)
  })
})
