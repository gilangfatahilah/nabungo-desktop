// Mode sembunyikan nominal (privacy mode), per perangkat (bukan setting backend). Penyimpanan bisa
// tidak tersedia (mode privat, situs diblokir): selalu jatuh ke default (nominal terlihat).

const KEY = 'nabungo.hideAmounts'

/** Pengganti angka saat disembunyikan; panjang tetap agar jumlah digit tidak bocor. */
export const MASK = '••••••••'
/** Pengganti label sumbu/nominal ringkas di grafik. */
export const MASK_SHORT = '•••'

export function loadAmountsHidden(): boolean {
  try {
    return localStorage.getItem(KEY) === '1'
  } catch {
    return false
  }
}

export function saveAmountsHidden(hidden: boolean) {
  try {
    localStorage.setItem(KEY, hidden ? '1' : '0')
  } catch {
    // Tidak tersimpan: preferensi hanya berlaku sampai aplikasi ditutup.
  }
}
