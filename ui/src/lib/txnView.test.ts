import { afterEach, describe, expect, it, vi } from 'vitest'
import { loadHiddenColumns, loadView, saveHiddenColumns, saveView } from './txnView'

describe('preferensi tampilan transaksi', () => {
  afterEach(() => {
    vi.restoreAllMocks()
    localStorage.clear()
  })

  it('default riwayat, tanpa kolom tersembunyi', () => {
    expect(loadView()).toBe('history')
    expect(loadHiddenColumns()).toEqual([])
  })

  it('menyimpan dan memuat ulang', () => {
    saveView('table')
    saveHiddenColumns(['tags', 'account'])
    expect(loadView()).toBe('table')
    expect(loadHiddenColumns()).toEqual(['account', 'tags'])
  })

  it('nilai rusak atau tak dikenal diabaikan', () => {
    localStorage.setItem('nabungo.txn.view', 'grid')
    localStorage.setItem('nabungo.txn.hiddenColumns', '{bukan json')
    expect(loadView()).toBe('history')
    expect(loadHiddenColumns()).toEqual([])
    localStorage.setItem('nabungo.txn.hiddenColumns', '["amount","tags"]')
    expect(loadHiddenColumns()).toEqual(['tags'])
  })

  it('storage tidak tersedia: jatuh ke default tanpa error', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('blocked') })
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('blocked') })
    expect(() => saveView('table')).not.toThrow()
    expect(loadView()).toBe('history')
    expect(loadHiddenColumns()).toEqual([])
  })
})
