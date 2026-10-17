import type { LocaleDto } from '@/ipc'

const pad = (n: number) => String(n).padStart(2, '0')
const iso = (y: number, m: number, d: number) => `${y}-${pad(m)}-${pad(d)}`
const intlLocale = (locale: LocaleDto) => (locale === 'id' ? 'id-ID' : 'en-US')

/** Tanggal lokal hari ini sebagai `YYYY-MM-DD` (bukan UTC: transaksi tidak punya jam). */
export function todayIso(now: Date = new Date()): string {
  return iso(now.getFullYear(), now.getMonth() + 1, now.getDate())
}

/** `YYYY-MM-DD` → "17 Okt 2026" / "Oct 17, 2026", tanpa geser zona waktu. */
export function formatDateInput(value: string, locale: LocaleDto): string {
  const [y, m, d] = value.split('-').map(Number)
  return new Intl.DateTimeFormat(intlLocale(locale), { day: 'numeric', month: 'short', year: 'numeric' })
    .format(new Date(y!, m! - 1, d!))
}

function valid(y: number, m: number, d: number): string | null {
  const dt = new Date(y, m - 1, d)
  return dt.getFullYear() === y && dt.getMonth() === m - 1 && dt.getDate() === d ? iso(y, m, d) : null
}

/** Nama bulan (pendek & panjang, kedua locale) → nomor bulan 1..12. */
const MONTHS: Map<string, number> = (() => {
  const map = new Map<string, number>()
  for (const loc of ['id-ID', 'en-US']) {
    for (const month of ['short', 'long'] as const) {
      const f = new Intl.DateTimeFormat(loc, { month })
      for (let i = 0; i < 12; i++) map.set(f.format(new Date(2026, i, 1)).toLowerCase().replace('.', ''), i + 1)
    }
  }
  return map
})()

const fullYear = (y: string | undefined, today: Date) =>
  y === undefined ? today.getFullYear() : y.length === 2 ? 2000 + Number(y) : Number(y)

/**
 * Teks ketikan pengguna → `YYYY-MM-DD`, atau null bila tidak dikenali/tidak valid.
 * Menerima ISO, angka (`17/10/2026`, `17-10`; hari-dulu untuk id, bulan-dulu untuk en),
 * dan nama bulan (`17 Okt 2026`, `Oct 17, 2026`). Tahun kosong = tahun berjalan.
 */
export function parseDateInput(text: string, locale: LocaleDto, today: Date = new Date()): string | null {
  const s = text.trim().toLowerCase()
  let m = /^(\d{4})-(\d{1,2})-(\d{1,2})$/.exec(s)
  if (m) return valid(Number(m[1]), Number(m[2]), Number(m[3]))

  m = /^(\d{1,2})[/.-](\d{1,2})(?:[/.-](\d{2}|\d{4}))?$/.exec(s)
  if (m) {
    const [a, b] = [Number(m[1]), Number(m[2])]
    const [day, month] = locale === 'id' ? [a, b] : [b, a]
    return valid(fullYear(m[3], today), month, day)
  }

  m = /^(\d{1,2})\s+([a-z]+)\.?(?:\s+(\d{2}|\d{4}))?$/.exec(s)
  if (m && MONTHS.has(m[2]!)) return valid(fullYear(m[3], today), MONTHS.get(m[2]!)!, Number(m[1]))

  m = /^([a-z]+)\.?\s+(\d{1,2}),?(?:\s+(\d{2}|\d{4}))?$/.exec(s)
  if (m && MONTHS.has(m[1]!)) return valid(fullYear(m[3], today), MONTHS.get(m[1]!)!, Number(m[2]))

  return null
}
