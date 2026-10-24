import { defineStore } from 'pinia'
import { ref } from 'vue'
import { toast } from 'vue-sonner'
import { i18n } from '@/i18n'
import * as ipc from '@/ipc'
import type { BudgetAlertDto, TxnRowDto, TxnSortDto, TxnSortFieldDto } from '@/ipc'
import { showBudgetAlerts } from '@/lib/budgetAlerts'
import type { TxnFilter } from '@/lib/txnFilter'
import { withErrorToast } from '@/lib/notify'
import { toDto, type TxnFormState } from '@/lib/txnForm'
import { loadHiddenColumns, loadView, type OptionalColumn, saveHiddenColumns, saveView, type TxnView } from '@/lib/txnView'
import { useAccountsStore } from './accounts'
import { useCategoriesStore } from './categories'

export type { TxnFilter } from '@/lib/txnFilter'

const PAGE = 100

export const EMPTY_FILTER: TxnFilter = {
  dateFrom: null, dateTo: null, accountId: null, categoryId: null, tag: null, payeeId: null,
  amountMin: null, amountMax: null, search: null,
}

export const DEFAULT_SORT: TxnSortDto = { field: 'date', desc: true }

// Daftar transaksi berhalaman. Setiap aksi tulis memuat ulang dari halaman pertama dan
// memuat ulang saldo akun: tidak ada saldo yang dihitung di UI.
export const useTransactionsStore = defineStore('transactions', () => {
  const query = ref<TxnFilter>({ ...EMPTY_FILTER })
  const sort = ref<TxnSortDto>({ ...DEFAULT_SORT })
  const view = ref<TxnView>(loadView())
  /** Kolom opsional yang disembunyikan di tampilan Tabel. */
  const hiddenColumns = ref<OptionalColumn[]>(loadHiddenColumns())
  const rows = ref<TxnRowDto[]>([])
  const total = ref(0)
  const loading = ref(false)
  /** Naik setiap transaksi disimpan/dihapus/dipulihkan: layar turunan (budget, laporan, alokasi) memuat ulang. */
  const ledgerVersion = ref(0)
  /** Peringatan budget dari penyimpanan terakhir. */

  /** `keepLoaded`: setelah aksi tulis, muat ulang sebanyak baris yang sudah dimuat agar posisi tidak hilang. */
  async function reload(keepLoaded = false) {
    loading.value = true
    try {
      const limit = keepLoaded ? Math.max(PAGE, rows.value.length) : PAGE
      const page = await ipc.listTxns({ ...query.value, sort: sort.value, limit, offset: 0 })
      rows.value = page.rows
      total.value = page.total
    } finally {
      loading.value = false
    }
  }

  async function loadMore() {
    if (loading.value || rows.value.length >= total.value) return
    loading.value = true
    try {
      const page = await ipc.listTxns({ ...query.value, sort: sort.value, limit: PAGE, offset: rows.value.length })
      const seen = new Set(rows.value.map((r) => r.id))
      rows.value = [...rows.value, ...page.rows.filter((r) => !seen.has(r.id))]
      total.value = page.total
    } finally {
      loading.value = false
    }
  }

  async function setQuery(patch: Partial<TxnFilter>) {
    query.value = { ...query.value, ...patch }
    await reload()
  }

  /** Klik kolom yang sama membalik arah; kolom baru mulai dari arah alaminya (teks A–Z, lainnya terbesar dulu). */
  async function setSort(field: TxnSortFieldDto) {
    sort.value = field === sort.value.field
      ? { field, desc: !sort.value.desc }
      : { field, desc: field !== 'payee' }
    await reload()
  }

  /** Riwayat dikelompokkan per tanggal, jadi selalu kembali ke urutan tanggal terbaru. */
  async function setView(v: TxnView) {
    view.value = v
    saveView(v)
    if (v === 'history' && (sort.value.field !== DEFAULT_SORT.field || sort.value.desc !== DEFAULT_SORT.desc)) {
      sort.value = { ...DEFAULT_SORT }
      await reload()
    }
  }

  function toggleColumn(col: OptionalColumn, shown: boolean) {
    hiddenColumns.value = shown ? hiddenColumns.value.filter((c) => c !== col) : [...new Set([...hiddenColumns.value, col])]
    saveHiddenColumns(hiddenColumns.value)
  }

  async function clear() {
    await setQuery({ ...EMPTY_FILTER })
  }

  async function refresh() {
    await Promise.all([reload(true), useAccountsStore().load()])
    ledgerVersion.value++
  }

  async function save(id: string | null, state: TxnFormState): Promise<string> {
    const dto = toDto(state)
    let saved: string
    let alerts: BudgetAlertDto[]
    if (id) {
      alerts = await ipc.updateTxn(id, dto)
      saved = id
    } else {
      ({ id: saved, alerts } = await ipc.createTxn(dto))
    }
    await refresh()
    await notifyAlerts(alerts)
    return saved
  }

  async function notifyAlerts(alerts: BudgetAlertDto[]) {
    if (!alerts.length) return
    // Transaksi sudah tersimpan: gagal memuat nama kategori tidak boleh membuat save tampak gagal.
    try {
      const categories = useCategoriesStore()
      if (!categories.loaded) await categories.load()
      showBudgetAlerts(alerts, new Map(categories.list.map((c) => [c.id, c.name])))
    } catch (e) {
      console.warn('gagal menampilkan peringatan budget', e)
    }
  }

  async function remove(id: string) {
    await ipc.deleteTxn(id)
    await refresh()
    const t = i18n.global.t
    toast(t('txn.deleted'), {
      action: {
        label: t('actions.undo'),
        onClick: () => void withErrorToast(async () => {
          await ipc.undoDeleteTxn(id)
          await refresh()
        }),
      },
    })
  }

  return {
    query, sort, view, hiddenColumns, rows, total, loading, ledgerVersion,
    reload, refresh, loadMore, setQuery, setSort, setView, toggleColumn, clear, save, remove, notifyAlerts,
  }
})
