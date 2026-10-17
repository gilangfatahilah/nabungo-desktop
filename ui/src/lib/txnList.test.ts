import { describe, expect, it } from 'vitest'
import type { TxnRowDto } from '@/ipc'
import { groupByDate } from './txnList'

const row = (id: string, date: string): TxnRowDto => ({
  id, kind: 'expense', date, payee: null, note: null, amount: -1, accountId: 'a', toAccountId: null,
  categoryId: null, splitCount: 1, fee: 0, tags: [], accountDelta: null, runningBalance: null,
})

describe('groupByDate', () => {
  it('menyisipkan header per tanggal baru dan mempertahankan urutan', () => {
    const items = groupByDate([row('1', '2026-10-12'), row('2', '2026-10-12'), row('3', '2026-10-11')])
    expect(items.map((i) => i.type)).toEqual(['date', 'row', 'row', 'date', 'row'])
    expect(items[0]).toMatchObject({ type: 'date', date: '2026-10-12' })
    expect(items[4]).toMatchObject({ type: 'row', row: { id: '3' } })
    expect(new Set(items.map((i) => i.key)).size).toBe(5)
  })

  it('kosong → kosong', () => {
    expect(groupByDate([])).toEqual([])
  })
})
