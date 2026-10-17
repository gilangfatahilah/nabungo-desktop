import { describe, expect, it } from 'vitest'
import { formatBp, parsePercent } from './percent'

describe('parsePercent', () => {
  it('menerima bilangan bulat, desimal koma/titik, dan tanda persen', () => {
    expect(parsePercent('60')).toBe(6000)
    expect(parsePercent('60,5')).toBe(6050)
    expect(parsePercent('60.5')).toBe(6050)
    expect(parsePercent(' 60% ')).toBe(6000)
    expect(parsePercent('0,25')).toBe(25)
    expect(parsePercent('100')).toBe(10000)
  })

  it('menolak lebih dari 2 desimal dan teks bukan angka', () => {
    expect(parsePercent('1,234')).toBeNull()
    expect(parsePercent('abc')).toBeNull()
    expect(parsePercent('')).toBeNull()
    expect(parsePercent('-5')).toBeNull()
  })
})

describe('formatBp', () => {
  it('menampilkan persen dengan pemisah desimal sesuai locale', () => {
    expect(formatBp(4200, 'id')).toBe('42%')
    expect(formatBp(4250, 'id')).toBe('42,5%')
    expect(formatBp(4250, 'en')).toBe('42.5%')
    expect(formatBp(-125, 'id')).toBe('-1,25%')
  })
})
