import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as ipc from '@/ipc'
import type { AllocationOverviewDto, BudgetOverviewDto, CategoryAmountDto, TxnRowDto } from '@/ipc'
import { todayIso } from '@/lib/date'
import { EMPTY_FILTER } from '@/stores/transactions'

/** Baris tabel "Transaksi terakhir" (docs/design/design.md §8.1). */
export const RECENT_LIMIT = 8
/** Kategori bernama di donut kartu "Pengeluaran periode ini"; sisanya digabung jadi "Lainnya". */
export const TOP_LIMIT = 5

// Data Dasbor di luar laporan (arus kas & kekayaan bersih ada di store reports); semua angka dari backend.
export const useDashboardStore = defineStore('dashboard', () => {
  const recent = ref<TxnRowDto[]>([])
  const budget = ref<BudgetOverviewDto | null>(null)
  const expenses = ref<CategoryAmountDto[]>([])
  const allocation = ref<AllocationOverviewDto | null>(null)

  async function load() {
    const period = await ipc.periodOf(todayIso())
    const [page, overview, breakdown, alloc] = await Promise.all([
      ipc.listTxns({ ...EMPTY_FILTER, sort: null, limit: RECENT_LIMIT, offset: 0 }),
      ipc.budgetOverview(period),
      ipc.categoryBreakdown(period, 'expense'),
      ipc.allocationOverview(),
    ])
    recent.value = page.rows
    budget.value = overview
    // Urutan dari backend: nominal turun, lalu nama.
    expenses.value = breakdown
    allocation.value = alloc
  }

  return { recent, budget, expenses, allocation, load }
})
