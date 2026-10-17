import { describe, expect, it } from 'vitest'
import { formatIdr, moneyParts } from './money'

describe('formatIdr', () => {
  it('formats rupiah with dot thousands separator', () => {
    expect(formatIdr(1250000).replace(/\s/g, ' ')).toBe('Rp 1.250.000')
  })

  it('formats negative amounts with a true minus sign', () => {
    expect(formatIdr(-25000).replace(/\s/g, ' ')).toBe('\u2212Rp 25.000')
  })

  it('rejects non-integer amounts', () => {
    expect(() => formatIdr(10.5)).toThrow(RangeError)
  })
})

describe('moneyParts', () => {
  it('splits sign, currency, and number', () => {
    expect(moneyParts(-25000, 'id')).toEqual({ sign: '\u2212', currency: 'Rp', number: '25.000' })
  })

  it('has no sign for zero and positive amounts', () => {
    expect(moneyParts(0, 'id').sign).toBe('')
    expect(moneyParts(1250000, 'id')).toEqual({ sign: '', currency: 'Rp', number: '1.250.000' })
  })

  it('follows the en locale grouping', () => {
    expect(moneyParts(1250000, 'en').number).toBe('1,250,000')
  })
})
