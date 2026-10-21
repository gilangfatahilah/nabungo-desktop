import { defineStore } from 'pinia'
import { ref } from 'vue'
import { toast } from 'vue-sonner'
import { i18n } from '@/i18n'
import * as ipc from '@/ipc'
import type { BudgetAlertDto, TxnRowDto } from '@/ipc'
import { showBudgetAlerts } from '@/lib/budgetAlerts'
import type { TxnFilter } from '@/lib/txnFilter'
import { withErrorToast } from '@/lib/notify'
import { toDto, type TxnFormState } from '@/lib/txnForm'
import { useAccountsStore } from './accounts'
import { useCategoriesStore } from './categories'

export type { TxnFilter } from '@/lib/txnFilter'

const PAGE = 100

export const EMPTY_FILTER: TxnFilter = {
  dateFrom: null, dateTo: null, accountId: null, categoryId: null, tag: null, payeeId: null,
  amountMin: null, amountMax: null, search: null,
}

// Daftar transaksi berhalaman. Setiap aksi tulis memuat ulang dari halaman pertama dan
// memuat ulang saldo akun: tidak ada saldo yang dihitung di UI.
export const useTransactionsStore = defineStore('transactions', () => {
  const query = ref<TxnFilter>({ ...EMPTY_FILTER })
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
      const page = await ipc.listTxns({ ...query.value, limit, offset: 0 })
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
      const page = await ipc.listTxns({ ...query.value, limit: PAGE, offset: rows.value.length })
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
    if (alerts.length) {
      // Transaksi sudah tersimpan: gagal memuat nama kategori tidak boleh membuat save tampak gagal.
      try {
        const categories = useCategoriesStore()
        if (!categories.loaded) await categories.load()
        showBudgetAlerts(alerts, new Map(categories.list.map((c) => [c.id, c.name])))
      } catch (e) {
        console.warn('gagal menampilkan peringatan budget', e)
      }
    }
    return saved
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

  return { query, rows, total, loading, ledgerVersion, reload, loadMore, setQuery, clear, save, remove }
})
