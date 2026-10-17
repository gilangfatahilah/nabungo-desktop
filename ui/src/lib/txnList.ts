import type { TxnRowDto } from '@/ipc'

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

/** `YYYY-MM-DD` → "12 Okt 2026" sesuai locale, tanpa geser zona waktu. */
export function formatDateHeader(iso: string, locale: string): string {
  const [y, m, d] = iso.split('-').map(Number)
  return new Intl.DateTimeFormat(locale === 'id' ? 'id-ID' : 'en-US', { day: 'numeric', month: 'short', year: 'numeric' })
    .format(new Date(y!, m! - 1, d!))
}
