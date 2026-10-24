import type { TxnQueryDto } from '@/ipc'

// Urutan bukan filter: tidak dihitung sebagai filter aktif dan tidak ikut "Hapus semua".
export type TxnFilter = Omit<TxnQueryDto, 'limit' | 'offset' | 'sort'>

/** Preset periode di toolbar Transaksi. Periode budget menyusul di Fase 5. */
export type Period = 'all' | 'thisMonth' | 'last30' | 'custom'

const pad = (n: number) => String(n).padStart(2, '0')

function shift(iso: string, days: number): string {
  const [y, m, d] = iso.split('-').map(Number)
  const dt = new Date(Date.UTC(y!, m! - 1, d! + days))
  return `${dt.getUTCFullYear()}-${pad(dt.getUTCMonth() + 1)}-${pad(dt.getUTCDate())}`
}

/** Rentang tanggal untuk preset; `custom` tidak mengubah rentang sehingga tidak didukung di sini. */
export function periodRange(p: Exclude<Period, 'custom'>, today: string): Pick<TxnFilter, 'dateFrom' | 'dateTo'> {
  if (p === 'all') return { dateFrom: null, dateTo: null }
  if (p === 'last30') return { dateFrom: shift(today, -29), dateTo: today }
  const [y, m] = today.split('-').map(Number)
  const last = new Date(Date.UTC(y!, m!, 0)).getUTCDate()
  return { dateFrom: `${y}-${pad(m!)}-01`, dateTo: `${y}-${pad(m!)}-${pad(last)}` }
}

export function periodOf(q: Pick<TxnFilter, 'dateFrom' | 'dateTo'>, today: string): Period {
  for (const p of ['all', 'thisMonth', 'last30'] as const) {
    const r = periodRange(p, today)
    if (r.dateFrom === q.dateFrom && r.dateTo === q.dateTo) return p
  }
  return 'custom'
}

export function activeFilterCount(q: TxnFilter): number {
  return Object.values(q).filter((v) => v !== null && v !== '').length
}
