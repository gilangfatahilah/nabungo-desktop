import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { useUiStore } from '@/stores/ui'
import { useMoneyFormat } from './useMoneyFormat'

beforeEach(() => {
  localStorage.clear()
  setActivePinia(createPinia())
})

describe('useMoneyFormat', () => {
  it('memformat nominal seperti lib/money saat terlihat', () => {
    const f = useMoneyFormat()
    expect(f.money(-1_250_000)).toBe('−Rp 1.250.000')
    expect(f.num(-1_250_000)).toBe('−1.250.000')
    expect(f.compact(1_250_000)).toBe('1,3\u00a0jt')
    expect(f.axis(1_249_999.6)).toBe('1,3\u00a0jt')
  })

  it('menyamarkan semua bentuk saat mode sembunyikan nominal aktif', () => {
    useUiStore().amountsHidden = true
    const f = useMoneyFormat()
    expect(f.money(-1_250_000)).toBe('Rp ••••••••')
    expect(f.num(-1_250_000)).toBe('••••••••')
    expect(f.compact(1_250_000)).toBe('•••')
    expect(f.axis(5_000)).toBe('•••')
  })
})
