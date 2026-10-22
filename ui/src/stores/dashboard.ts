import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as ipc from '@/ipc'
import type { BudgetOverviewDto, TxnRowDto } from '@/ipc'
import { todayIso } from '@/lib/date'
import { EMPTY_FILTER } from '@/stores/transactions'

/** Baris tabel "Transaksi terakhir" (docs/design/design.md §8.1). */
export const RECENT_LIMIT = 8

// Data Dasbor di luar laporan (arus kas & kekayaan bersih ada di store reports); semua angka dari backend.
export const useDashboardStore = defineStore('dashboard', () => {
  const recent = ref<TxnRowDto[]>([])
  const budget = ref<BudgetOverviewDto | null>(null)

  async function load() {
    const period = await ipc.periodOf(todayIso())
    const [page, overview] = await Promise.all([
      ipc.listTxns({ ...EMPTY_FILTER, limit: RECENT_LIMIT, offset: 0 }),
      ipc.budgetOverview(period),
    ])
    recent.value = page.rows
    budget.value = overview
  }

  return { recent, budget, load }
})
