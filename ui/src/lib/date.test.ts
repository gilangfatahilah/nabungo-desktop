import { describe, expect, it } from 'vitest'
import { todayIso } from './date'

describe('todayIso', () => {
  it('memakai tanggal lokal dengan nol di depan', () => {
    expect(todayIso(new Date(2026, 0, 5, 23, 59))).toBe('2026-01-05')
  })
})
