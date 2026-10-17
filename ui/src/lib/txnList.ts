import type { LocaleDto, TxnRowDto } from '@/ipc'
import { formatDateInput } from '@/lib/date'

export type ListItem = { type: 'date'; date: string; key: string } | { type: 'row'; row: TxnRowDto; key: string }

/** Sisipkan header setiap kali tanggal berganti; urutan baris dari backend dipertahankan. */
export function groupByDate(rows: TxnRowDto[]): ListItem[] {
  const items: ListItem[] = []
  let last: string | null = null
  for (const row of rows) {
    if (row.date !== last) {
      items.push({ type: 'date', date: row.date, key: `d:${row.date}:${items.length}` })
      last = row.date
    }
    items.push({ type: 'row', row, key: row.id })
  }
  return items
}

/** `YYYY-MM-DD` → "16 Okt 2026, Jumat" sesuai locale, tanpa geser zona waktu. */
export function formatDateHeader(iso: string, locale: LocaleDto): string {
  const [y, m, d] = iso.split('-').map(Number)
  const weekday = new Intl.DateTimeFormat(locale === 'id' ? 'id-ID' : 'en-US', { weekday: 'long' }).format(new Date(y!, m! - 1, d!))
  return `${formatDateInput(iso, locale)}, ${weekday}`
}
