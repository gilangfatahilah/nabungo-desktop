import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import * as ipc from '@/ipc'
import type { CashflowPointDto, CategoryComparisonDto, CategoryKindDto, CategoryTrendPointDto, NetWorthPointDto } from '@/ipc'
import { todayIso } from '@/lib/date'
import { loadRange, type ReportRange, saveRange } from '@/lib/reportStats'

// Jumlah periode cashflow & kekayaan bersih untuk Dasbor.
export const REPORT_PERIODS = 12

// Semua angka dihitung backend; store hanya menyimpan pilihan rentang, periode fokus, jenis,
// dan kategori tren. `load` melayani Dasbor (12 periode tetap); `loadReport` layar Laporan.
export const useReportsStore = defineStore('reports', () => {
  const cashflow = ref<CashflowPointDto[]>([])
  const netWorth = ref<NetWorthPointDto[]>([])

  /** Periode berjalan; jendela chart Laporan selalu berakhir di sini. */
  const current = ref<string | null>(null)
  const range = ref<ReportRange>(loadRange())
  /** `range` periode + satu sebelumnya (pembanding periode pertama), maks. 24 total. */
  const rangeCashflow = ref<CashflowPointDto[]>([])
  const rangeNetWorth = ref<NetWorthPointDto[]>([])
  /** Periode fokus untuk KPI dan kategori; selalu di dalam jendela. */
  const period = ref<string | null>(null)
  const kind = ref<CategoryKindDto>('expense')
  const comparison = ref<CategoryComparisonDto[]>([])
  const trendCategoryId = ref<string | null>(null)
  /** Jenis saat kategori tren dipilih; ganti jenis mereset pilihan. */
  const trendKind = ref<CategoryKindDto | null>(null)
  const trend = ref<CategoryTrendPointDto[]>([])

  const span = computed(() => rangeCashflow.value.slice(-range.value))
  const focusIndex = computed(() => rangeCashflow.value.findIndex((p) => p.period === period.value))
  const focus = computed(() => rangeCashflow.value[focusIndex.value] ?? null)
  const previous = computed(() => (focusIndex.value > 0 ? rangeCashflow.value[focusIndex.value - 1]! : null))
  const canPrev = computed(() => span.value.length > 0 && span.value[0]!.period !== period.value)
  const canNext = computed(() => period.value !== null && period.value !== current.value)

  async function load() {
    const cur = await ipc.periodOf(todayIso())
    const [c, n] = await Promise.all([
      ipc.cashflowReport(cur, REPORT_PERIODS),
      ipc.netWorthHistory(cur, REPORT_PERIODS),
    ])
    cashflow.value = c
    netWorth.value = n
  }

  async function loadTrend() {
    trend.value = current.value && trendCategoryId.value
      ? await ipc.categoryTrend(current.value, range.value, trendCategoryId.value)
      : []
  }

  async function loadFocus() {
    if (!period.value) return
    comparison.value = await ipc.categoryComparison(period.value, kind.value)
    if (!trendCategoryId.value || trendKind.value !== kind.value) {
      trendCategoryId.value = comparison.value[0]?.categoryId ?? null
      trendKind.value = kind.value
    }
    await loadTrend()
  }

  async function loadReport() {
    const cur = await ipc.periodOf(todayIso())
    current.value = cur
    const [c, n] = await Promise.all([
      ipc.cashflowReport(cur, range.value + 1),
      ipc.netWorthHistory(cur, range.value),
    ])
    rangeCashflow.value = c
    rangeNetWorth.value = n
    if (!span.value.some((p) => p.period === period.value)) period.value = cur
    await loadFocus()
  }

  async function select(p: string) {
    period.value = p
    await loadFocus()
  }

  async function shift(step: -1 | 1) {
    const i = span.value.findIndex((p) => p.period === period.value) + step
    const target = span.value[i]
    if (target) await select(target.period)
  }

  async function setRange(r: ReportRange) {
    range.value = r
    saveRange(r)
    await loadReport()
  }

  async function setKind(k: CategoryKindDto) {
    kind.value = k
    await loadFocus()
  }

  async function setTrendCategory(id: string) {
    trendCategoryId.value = id
    trendKind.value = kind.value
    await loadTrend()
  }

  return {
    cashflow, netWorth, load,
    current, range, rangeCashflow, rangeNetWorth, period, kind, comparison, trendCategoryId, trend,
    span, focus, previous, canPrev, canNext,
    loadReport, select, shift, setRange, setKind, setTrendCategory,
  }
})
