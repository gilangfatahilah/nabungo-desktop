import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as ipc from '@/ipc'
import type { BudgetOverviewDto } from '@/ipc'
import { todayIso } from '@/lib/date'

export const useBudgetStore = defineStore('budget', () => {
  const period = ref<string | null>(null)
  const overview = ref<BudgetOverviewDto | null>(null)

  async function load(p: string) {
    overview.value = await ipc.budgetOverview(p)
    period.value = p
  }

  async function init() {
    await load(await ipc.periodOf(todayIso()))
  }

  /** Tetangga periode berasal dari backend (aturan tanggal mulai ada di Rust). */
  async function go(dir: 'prev' | 'next') {
    if (overview.value) await load(overview.value[dir])
  }

  async function then<T>(p: Promise<T>): Promise<T> {
    const r = await p
    if (period.value) await load(period.value)
    return r
  }

  return {
    period,
    overview,
    init,
    load,
    go,
    setAmount: (categoryId: string, amount: number | null) => then(ipc.setBudget(categoryId, period.value!, amount)),
    setRollover: (categoryId: string, on: boolean) => then(ipc.setBudgetRollover(categoryId, period.value!, on)),
    copyPrevious: () => then(ipc.copyBudgets(period.value!)),
  }
})
