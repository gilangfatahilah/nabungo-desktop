import { describe, expect, it } from 'vitest'
import { formatTrend, trendPercent } from './trend'

describe('trendPercent', () => {
  it('persen bulat terhadap nilai absolut periode lalu', () => {
    expect(trendPercent(112, 100)).toBe(12)
    expect(trendPercent(96, 100)).toBe(-4)
    expect(trendPercent(-50, -100)).toBe(50)
  })

  it('tanpa pembanding → null', () => {
    expect(trendPercent(100, 0)).toBeNull()
    expect(trendPercent(100, undefined)).toBeNull()
  })
})

describe('formatTrend', () => {
  it('tanda selalu tertulis', () => {
    expect(formatTrend(12)).toBe('+12%')
    expect(formatTrend(-4)).toBe('−4%')
    expect(formatTrend(0)).toBe('0%')
  })
})
