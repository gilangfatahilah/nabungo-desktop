import type { ShortcutAction } from '@/lib/shortcuts'

/** Abaikan navigasi/palette saat sheet transaksi baru terbuka. */
export function shouldIgnoreAction(a: ShortcutAction, newTxnOpen: boolean): boolean {
  return newTxnOpen && (a.kind === 'navigate' || a.kind === 'palette')
}
