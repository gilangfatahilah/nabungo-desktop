<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import EmptyState from '@/components/EmptyState.vue'
import PageHeader from '@/components/PageHeader.vue'
import BarChart from '@/components/charts/BarChart.vue'
import LineChart from '@/components/charts/LineChart.vue'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import type { CategoryKindDto } from '@/ipc'
import { colorClass } from '@/lib/colors'
import { formatPeriod } from '@/lib/date'
import { iconFor } from '@/lib/icons'
import { formatCompact, moneyParts } from '@/lib/money'
import { withErrorToast } from '@/lib/notify'
import { formatBp } from '@/lib/percent'
import { cn } from '@/lib/utils'
import { useCategoriesStore } from '@/stores/categories'
import { REPORT_PERIODS, useReportsStore } from '@/stores/reports'
import { useSettingsStore } from '@/stores/settings'

const store = useReportsStore()
const categories = useCategoriesStore()
const settings = useSettingsStore()
const { t } = useI18n()

const KINDS: CategoryKindDto[] = ['expense', 'income']

const num = (v: number) => {
  const p = moneyParts(v, settings.locale)
  return p.sign + p.number
}
// Label sumbu ringkas agar nominal jutaan tidak terpotong; nilai lengkap ada di tabel.
const axis = (v: number) => formatCompact(Math.round(v), settings.locale)
const label = (period: string) => formatPeriod(period, settings.locale)

const cashflowLabels = computed(() => store.cashflow.map((p) => label(p.period)))
const cashflowSeries = computed(() => [
  { key: 'income', label: t('reports.inflow'), color: 'var(--chart-2)', values: store.cashflow.map((p) => p.income) },
  { key: 'expense', label: t('reports.outflow'), color: 'var(--chart-3)', values: store.cashflow.map((p) => p.expense) },
])
const selectedIndex = computed(() => store.cashflow.findIndex((p) => p.period === store.period))
const select = (period: string) => withErrorToast(() => store.select(period))

const byId = computed(() => new Map(categories.list.map((c) => [c.id, c])))
const breakdownTotal = computed(() => store.breakdown.reduce((sum, b) => sum + b.amount, 0))
// Porsi dalam basis point; hanya untuk tampilan, nominalnya tetap dari backend.
const shareBp = (amount: number) => (breakdownTotal.value ? Math.round((amount * 10000) / breakdownTotal.value) : 0)
const setKind = (k: unknown) => withErrorToast(() => store.setKind(k as CategoryKindDto))

const netWorthLabels = computed(() => store.netWorth.map((p) => label(p.period)))
const netWorthSeries = computed(() => [
  { key: 'net', label: t('reports.net'), color: 'var(--chart-1)', values: store.netWorth.map((p) => p.net) },
  { key: 'assets', label: t('reports.assets'), color: 'var(--chart-2)', values: store.netWorth.map((p) => p.assets), dashed: true },
  { key: 'liabilities', label: t('reports.liabilities'), color: 'var(--chart-3)', values: store.netWorth.map((p) => p.liabilities), dashed: true },
])

const row = 'grid grid-cols-[8rem_repeat(3,minmax(7rem,10rem))] items-center gap-4 px-2'

onMounted(() => {
  void withErrorToast(() => Promise.all([store.load(), categories.load()]))
})
</script>

<template>
  <PageHeader :title="t('nav.reports')">{{ t('reports.lastPeriods', { count: REPORT_PERIODS }) }}</PageHeader>

  <div class="flex flex-col gap-10">
    <section class="flex flex-col gap-4">
      <div class="flex max-w-3xl items-center justify-between gap-4">
        <h2 class="font-medium">{{ t('reports.cashflow') }}</h2>
        <ul class="flex gap-4 text-meta text-muted-foreground">
          <li v-for="s in cashflowSeries" :key="s.key" class="flex items-center gap-2">
            <span class="size-2.5 rounded-full" :style="{ background: s.color }" />{{ s.label }}
          </li>
        </ul>
      </div>
      <div class="max-w-3xl" data-testid="cashflow-chart">
        <BarChart
          :labels="cashflowLabels" :series="cashflowSeries" :label="t('reports.cashflowChart')" :format="axis"
          :selected="selectedIndex" @select="(i) => store.cashflow[i] && select(store.cashflow[i]!.period)"
        />
      </div>
      <div class="-mx-2 flex max-w-3xl flex-col" role="table">
        <div :class="cn(row, 'h-8 text-meta text-muted-foreground')" role="row">
          <span role="columnheader">{{ t('reports.period') }}</span>
          <span role="columnheader" class="text-right">{{ t('reports.inflow') }}</span>
          <span role="columnheader" class="text-right">{{ t('reports.outflow') }}</span>
          <span role="columnheader" class="text-right">{{ t('reports.net') }}</span>
        </div>
        <div
          v-for="(p, i) in store.cashflow" :key="p.period" role="row" tabindex="0" :data-testid="`cashflow-row-${p.period}`"
          :aria-selected="p.period === store.period"
          :class="cn(row, 'h-9 cursor-pointer rounded-lg outline-none hover:bg-muted/50 focus-visible:ring-2 focus-visible:ring-ring', p.period === store.period && 'bg-muted')"
          @click="select(p.period)" @keydown.enter.prevent="select(p.period)" @keydown.space.prevent="select(p.period)"
        >
          <span role="cell" class="num">{{ cashflowLabels[i] }}</span>
          <span role="cell" class="num text-right">{{ num(p.income) }}</span>
          <span role="cell" class="num text-right">{{ num(p.expense) }}</span>
          <span role="cell" data-part="net" :class="cn('num text-right', p.net < 0 && 'text-destructive')">{{ num(p.net) }}</span>
        </div>
      </div>
    </section>

    <section class="flex flex-col gap-4">
      <div class="flex max-w-3xl items-center justify-between gap-4">
        <h2 class="font-medium" data-testid="breakdown-title">
          {{ t('reports.breakdown', { period: store.period ? label(store.period) : '' }) }}
        </h2>
        <Tabs :model-value="store.kind" @update:model-value="setKind">
          <TabsList>
            <TabsTrigger v-for="k in KINDS" :key="k" :value="k" class="px-3">{{ t(`reports.${k}`) }}</TabsTrigger>
          </TabsList>
        </Tabs>
      </div>
      <EmptyState
        v-if="!store.breakdown.length" data-testid="breakdown-empty"
        :title="store.kind === 'income' ? t('reports.emptyIncome') : t('reports.emptyExpense')"
      />
      <ul v-else class="-mx-2 flex max-w-3xl flex-col" role="table">
        <li
          v-for="b in store.breakdown" :key="b.categoryId" data-testid="breakdown-row" role="row"
          class="grid h-12 grid-cols-[minmax(0,1fr)_8rem_5rem_8rem] items-center gap-4 rounded-lg px-2 hover:bg-muted/50"
        >
          <span role="cell" class="flex min-w-0 items-center gap-3">
            <span :class="cn('flex size-7 shrink-0 items-center justify-center rounded-md', colorClass(byId.get(b.categoryId)?.color ?? null))">
              <component :is="iconFor(byId.get(b.categoryId)?.icon ?? null)" class="size-4" />
            </span>
            <span data-part="name" class="truncate">{{ byId.get(b.categoryId)?.name ?? '' }}</span>
          </span>
          <span role="cell" class="num text-right">{{ num(b.amount) }}</span>
          <span role="cell" class="num text-right text-muted-foreground">{{ formatBp(shareBp(b.amount), settings.locale) }}</span>
          <span aria-hidden="true" class="h-1.5 overflow-hidden rounded-full bg-muted">
            <span class="block h-full rounded-full" :style="{ width: `${shareBp(b.amount) / 100}%`, background: 'var(--chart-1)' }" />
          </span>
        </li>
      </ul>
    </section>

    <section class="flex flex-col gap-4">
      <div class="flex max-w-3xl items-center justify-between gap-4">
        <h2 class="font-medium">{{ t('reports.netWorth') }}</h2>
        <ul class="flex gap-4 text-meta text-muted-foreground">
          <li v-for="s in netWorthSeries" :key="s.key" class="flex items-center gap-2">
            <span class="size-2.5 rounded-full" :style="{ background: s.color }" />{{ s.label }}
          </li>
        </ul>
      </div>
      <div class="max-w-3xl" data-testid="networth-chart">
        <LineChart :labels="netWorthLabels" :series="netWorthSeries" :label="t('reports.netWorthChart')" :format="axis" />
      </div>
      <div class="-mx-2 flex max-w-3xl flex-col" role="table">
        <div :class="cn(row, 'h-8 text-meta text-muted-foreground')" role="row">
          <span role="columnheader">{{ t('reports.period') }}</span>
          <span role="columnheader" class="text-right">{{ t('reports.assets') }}</span>
          <span role="columnheader" class="text-right">{{ t('reports.liabilities') }}</span>
          <span role="columnheader" class="text-right">{{ t('reports.net') }}</span>
        </div>
        <div v-for="(p, i) in store.netWorth" :key="p.period" role="row" :data-testid="`networth-row-${p.period}`" :class="cn(row, 'h-9')">
          <span role="cell" class="num">{{ netWorthLabels[i] }}</span>
          <span role="cell" class="num text-right">{{ num(p.assets) }}</span>
          <span role="cell" class="num text-right">{{ num(p.liabilities) }}</span>
          <span role="cell" :class="cn('num text-right', p.net < 0 && 'text-destructive')">{{ num(p.net) }}</span>
        </div>
      </div>
    </section>
  </div>
</template>
