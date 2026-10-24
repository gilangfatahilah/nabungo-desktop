import type { TxnRowDto } from '@/ipc'
import { withErrorToast } from '@/lib/notify'
import { isEditable } from '@/lib/txnForm'
import { useTransactionsStore } from '@/stores/transactions'
import { useUiStore } from '@/stores/ui'

/** Aksi baris transaksi (menu `⋯` & keyboard), sama di tampilan Riwayat dan Tabel (docs/design/design.md §7.5). */
export function useTxnRowActions() {
  const store = useTransactionsStore()
  const ui = useUiStore()

  function edit(r: TxnRowDto) {
    if (isEditable(r.kind)) ui.openEditTxn(r.id)
  }

  function duplicate(r: TxnRowDto) {
    if (isEditable(r.kind)) ui.openDuplicateTxn(r.id)
  }

  function makeRecurring(r: TxnRowDto) {
    if (isEditable(r.kind)) ui.openRuleEditor({ mode: 'fromTxn', txnId: r.id })
  }

  function remove(r: TxnRowDto) {
    void withErrorToast(() => store.remove(r.id))
  }

  /** `Enter` edit, `Ctrl+D` duplikat, `Delete` hapus, `↑↓` pindah baris lewat `move`. */
  function onKey(e: KeyboardEvent, r: TxnRowDto, move: (from: HTMLElement, delta: number) => void) {
    if (e.target !== e.currentTarget) return
    if (e.key === 'Enter') edit(r)
    else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'd') duplicate(r)
    else if (e.key === 'Delete') remove(r)
    else if (e.key === 'ArrowDown') move(e.currentTarget as HTMLElement, 1)
    else if (e.key === 'ArrowUp') move(e.currentTarget as HTMLElement, -1)
    else return
    e.preventDefault()
  }

  return { edit, duplicate, makeRecurring, remove, onKey }
}
