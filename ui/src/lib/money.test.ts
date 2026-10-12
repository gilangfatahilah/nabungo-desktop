import { describe, expect, it } from 'vitest'
import { formatIdr } from './money'

describe('formatIdr', () => {
  it('formats rupiah with dot thousands separator', () => {
    expect(formatIdr(1250000).replace(/\s/g, ' ')).toBe('Rp 1.250.000')
  })

  it('formats negative amounts', () => {
    expect(formatIdr(-25000).replace(/\s/g, ' ')).toBe('-Rp 25.000')
  })

  it('rejects non-integer amounts', () => {
    expect(() => formatIdr(10.5)).toThrow(RangeError)
  })
})
