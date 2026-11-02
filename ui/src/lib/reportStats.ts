// Statistik presentasi layar Laporan: hanya membagi/membandingkan angka dari backend,
// tidak pernah menjumlah posting sendiri (docs/design/design.md §7.11, §8.8).
import type { CategoryComparisonDto } from '@/ipc'

/** Rentang chart Laporan dalam jumlah periode. */
export const REPORT_RANGES = [3, 6, 12, 24] as const
export type ReportRange = (typeof REPORT_RANGES)[number]

const RANGE_KEY = 'nabungo.reports.range'

/** Rentang tersimpan per perangkat; penyimpanan yang tidak tersedia jatuh ke 12. */
export function loadRange(): ReportRange {
  try {
    const n = Number(localStorage.getItem(RANGE_KEY))
    return REPORT_RANGES.find((r) => r === n) ?? 12
  } catch {
    return 12
  }
}

export function saveRange(r: ReportRange) {
  try {
    localStorage.setItem(RANGE_KEY, String(r))
  } catch {
    // Tidak tersimpan: rentang hanya berlaku sampai aplikasi ditutup.
  }
}

/** Rasio tabungan (sisa ÷ pemasukan) dalam persen bulat; `null` bila tidak ada pemasukan. */
export function savingsRate(income: number, net: number): number | null {
  return income > 0 ? Math.round((net * 100) / income) : null
}

const dayNumber = (iso: string) => Date.UTC(Number(iso.slice(0, 4)), Number(iso.slice(5, 7)) - 1, Number(iso.slice(8, 10))) / 86_400_000

/**
 * Rata-rata per hari dalam periode `[from, to]` (inklusif). Periode yang sedang berjalan dibagi
 * hari yang sudah lewat (termasuk hari ini), bukan seluruh panjang periode.
 */
export function perDay(amount: number, from: string, to: string, today: string): number {
  const end = today >= from && today <= to ? today : to
  const days = dayNumber(end) - dayNumber(from) + 1
  return days > 0 ? Math.round(amount / days) : 0
}

/** Rata-rata bulat; `null` untuk daftar kosong. */
export function average(values: number[]): number | null {
  return values.length ? Math.round(values.reduce((s, v) => s + v, 0) / values.length) : null
}

export interface CategoryChange {
  categoryId: string
  amount: number
  previous: number
  /** `amount − previous`. */
  delta: number
}

/** Kenaikan dan penurunan terbesar (maks `n` masing-masing) dibanding periode lalu. */
export function biggestChanges(rows: CategoryComparisonDto[], n: number): { up: CategoryChange[]; down: CategoryChange[] } {
  const changes = rows
    .map((r) => ({ categoryId: r.categoryId, amount: r.amount, previous: r.previous, delta: r.amount - r.previous }))
    .filter((c) => c.delta !== 0)
  return {
    up: changes.filter((c) => c.delta > 0).sort((a, b) => b.delta - a.delta).slice(0, n),
    down: changes.filter((c) => c.delta < 0).sort((a, b) => a.delta - b.delta).slice(0, n),
  }
}
