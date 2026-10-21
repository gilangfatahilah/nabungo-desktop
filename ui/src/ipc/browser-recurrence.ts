// Port `crates/core/src/recurrence.rs` untuk mode browser: anchor (hari/tanggal) diambil dari
// tanggal mulai, tanggal di luar panjang bulan di-clamp ke akhir bulan. Ubah keduanya bersamaan.
import type { RecurrenceDto } from './bindings'

const pad = (n: number) => String(n).padStart(2, '0')
const parts = (s: string) => s.split('-').map(Number) as [number, number, number]
const fmt = (y: number, m: number, d: number) => `${String(y).padStart(4, '0')}-${pad(m)}-${pad(d)}`
const monthLength = (y: number, m: number) => new Date(Date.UTC(y, m, 0)).getUTCDate()
const clamped = (y: number, m: number, day: number) => fmt(y, m, Math.min(day, monthLength(y, m)))

function addDays(s: string, n: number): string {
  const [y, m, d] = parts(s)
  const t = new Date(Date.UTC(y, m - 1, d + n))
  return fmt(t.getUTCFullYear(), t.getUTCMonth() + 1, t.getUTCDate())
}

/** Kejadian sesudah `d`; `anchor` = tanggal mulai jadwal. `null` bila melewati `until`. */
export function nextAfter(rec: RecurrenceDto, anchor: string, d: string): string | null {
  const n = rec.interval
  const [y, m] = parts(d)
  const [, am, ad] = parts(anchor)
  let next: string
  switch (rec.freq) {
    case 'daily': next = addDays(d, n); break
    case 'weekly': next = addDays(d, 7 * n); break
    case 'monthly': {
      const months = y * 12 + (m - 1) + n
      next = clamped(Math.floor(months / 12), (months % 12) + 1, ad)
      break
    }
    case 'yearly': next = clamped(y + n, am, ad); break
  }
  return rec.until !== null && next > rec.until ? null : next
}

/**
 * Port `Recurrence::keep_day_of`: ubah jadwal dari tanggal akhir bulan yang ter-clamp
 * mempertahankan tanggal anchor lama (frekuensi, dan bulan untuk tahunan, harus sama).
 * Anchor hanya dibaca per bagian, jadi `2026-02-31` sah di sini.
 */
export function keepDay(rec: RecurrenceDto, start: string, old: RecurrenceDto, oldAnchor: string): string {
  const [y, m, d] = parts(start)
  const [, om, od] = parts(oldAnchor)
  const sameMonth = rec.freq === 'monthly' || (rec.freq === 'yearly' && m === om)
  if (rec.freq !== old.freq || !sameMonth || d !== monthLength(y, m) || od <= d) return start
  return fmt(y, m, od)
}

/** Hingga `n` kejadian pertama mulai `start` (inklusif). */
export function occurrences(rec: RecurrenceDto, start: string, n: number): string[] {
  const out: string[] = []
  let cur: string | null = rec.until === null || start <= rec.until ? start : null
  while (cur !== null && out.length < n) {
    out.push(cur)
    cur = nextAfter(rec, start, cur)
  }
  return out
}
