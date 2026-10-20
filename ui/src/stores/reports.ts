import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as ipc from '@/ipc'
import type { CashflowPointDto, CategoryAmountDto, CategoryKindDto, NetWorthPointDto } from '@/ipc'
import { todayIso } from '@/lib/date'

// Jumlah periode pada cashflow & kekayaan bersih.
export const REPORT_PERIODS = 12

// Semua angka dihitung backend; store hanya menyimpan pilihan periode & jenis breakdown.
export const useReportsStore = defineStore('reports', () => {
  const cashflow = ref<CashflowPointDto[]>([])
  const netWorth = ref<NetWorthPointDto[]>([])
  const breakdown = ref<CategoryAmountDto[]>([])
  const period = ref<string | null>(null)
  const kind = ref<CategoryKindDto>('expense')

  async function loadBreakdown() {
    if (period.value) breakdown.value = await ipc.categoryBreakdown(period.value, kind.value)
  }

  async function load() {
    const current = await ipc.periodOf(todayIso())
    const [c, n] = await Promise.all([
      ipc.cashflowReport(current, REPORT_PERIODS),
      ipc.netWorthHistory(current, REPORT_PERIODS),
    ])
    cashflow.value = c
    netWorth.value = n
    period.value ??= current
    await loadBreakdown()
  }

  async function select(p: string) {
    period.value = p
    await loadBreakdown()
  }

  async function setKind(k: CategoryKindDto) {
    kind.value = k
    await loadBreakdown()
  }

  return { cashflow, netWorth, breakdown, period, kind, load, select, setKind }
})
