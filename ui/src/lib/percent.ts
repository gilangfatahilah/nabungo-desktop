import type { LocaleDto } from '@/ipc'

const PATTERN = /^(\d{1,3})(?:[.,](\d{1,2}))?%?$/

/** Teks persen ("60", "60,5", "60.5", "60%") → basis point (10000 = 100%); maks 2 desimal. */
export function parsePercent(text: string): number | null {
  const m = PATTERN.exec(text.trim())
  if (!m) return null
  return Number(m[1]) * 100 + Number((m[2] ?? '').padEnd(2, '0'))
}

/** Basis point → "42%" / "42,5%" (id) / "42.5%" (en). */
export function formatBp(bp: number, locale: LocaleDto): string {
  const f = new Intl.NumberFormat(locale === 'id' ? 'id-ID' : 'en-US', { maximumFractionDigits: 2 })
  return `${f.format(bp / 100)}%`
}
