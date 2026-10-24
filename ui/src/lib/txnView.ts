// Preferensi tampilan layar Transaksi, per perangkat (bukan setting backend). Penyimpanan bisa
// tidak tersedia (mode privat, situs diblokir): selalu jatuh ke default.

export type TxnView = 'history' | 'table'

/** Kolom tabel yang bisa disembunyikan; Tanggal, Deskripsi, dan Nominal selalu tampil. */
export const OPTIONAL_COLUMNS = ['category', 'account', 'tags'] as const
export type OptionalColumn = (typeof OPTIONAL_COLUMNS)[number]

const VIEW_KEY = 'nabungo.txn.view'
const HIDDEN_KEY = 'nabungo.txn.hiddenColumns'

function read(key: string): string | null {
  try {
    return localStorage.getItem(key)
  } catch {
    return null
  }
}

function write(key: string, value: string) {
  try {
    localStorage.setItem(key, value)
  } catch {
    // Tidak tersimpan: preferensi hanya berlaku sampai aplikasi ditutup.
  }
}

export function loadView(): TxnView {
  return read(VIEW_KEY) === 'table' ? 'table' : 'history'
}

export function saveView(v: TxnView) {
  write(VIEW_KEY, v)
}

export function loadHiddenColumns(): OptionalColumn[] {
  try {
    const parsed: unknown = JSON.parse(read(HIDDEN_KEY) ?? '[]')
    return Array.isArray(parsed) ? OPTIONAL_COLUMNS.filter((c) => parsed.includes(c)) : []
  } catch {
    return []
  }
}

export function saveHiddenColumns(cols: OptionalColumn[]) {
  write(HIDDEN_KEY, JSON.stringify(cols))
}
