import { describe, expect, it } from 'vitest'
import { formatDateInput, parseDateInput, todayIso } from './date'

describe('todayIso', () => {
  it('memakai tanggal lokal dengan nol di depan', () => {
    expect(todayIso(new Date(2026, 0, 5, 23, 59))).toBe('2026-01-05')
  })
})

describe('formatDateInput', () => {
  it('menampilkan tanggal pendek sesuai locale', () => {
    expect(formatDateInput('2026-10-17', 'id')).toBe('17 Okt 2026')
    expect(formatDateInput('2026-10-17', 'en')).toBe('Oct 17, 2026')
  })
})

describe('parseDateInput', () => {
  const today = new Date(2026, 9, 17)

  it('menerima format tampilan kembali (round-trip)', () => {
    expect(parseDateInput('17 Okt 2026', 'id', today)).toBe('2026-10-17')
    expect(parseDateInput('Oct 17, 2026', 'en', today)).toBe('2026-10-17')
  })

  it('menerima nama bulan panjang dan huruf apa saja', () => {
    expect(parseDateInput('3 agustus 2025', 'id', today)).toBe('2025-08-03')
  })

  it('angka hari-dulu untuk id, bulan-dulu untuk en', () => {
    expect(parseDateInput('17/10/2026', 'id', today)).toBe('2026-10-17')
    expect(parseDateInput('10/17/2026', 'en', today)).toBe('2026-10-17')
    expect(parseDateInput('5-1-26', 'id', today)).toBe('2026-01-05')
  })

  it('tahun boleh dihilangkan (tahun berjalan)', () => {
    expect(parseDateInput('5/1', 'id', today)).toBe('2026-01-05')
    expect(parseDateInput('5 Jan', 'id', today)).toBe('2026-01-05')
  })

  it('menerima ISO apa adanya', () => {
    expect(parseDateInput('2026-02-28', 'id', today)).toBe('2026-02-28')
  })

  it('menolak tanggal tidak valid atau teks acak', () => {
    expect(parseDateInput('31/02/2026', 'id', today)).toBeNull()
    expect(parseDateInput('', 'id', today)).toBeNull()
    expect(parseDateInput('kemarin sore', 'id', today)).toBeNull()
  })
})
