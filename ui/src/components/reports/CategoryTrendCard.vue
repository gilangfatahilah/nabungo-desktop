<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import MoneyText from '@/components/MoneyText.vue'
import TableScroll from '@/components/TableScroll.vue'
import BarChart from '@/components/charts/BarChart.vue'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import CategorySelect from '@/components/CategorySelect.vue'
import { useMoneyFormat } from '@/composables/useMoneyFormat'
import { formatPeriod } from '@/lib/date'
import { withErrorToast } from '@/lib/notify'
import { average } from '@/lib/reportStats'
import { cn } from '@/lib/utils'
import { useCategoriesStore } from '@/stores/categories'
import { useReportsStore } from '@/stores/reports'
import { useSettingsStore } from '@/stores/settings'
import TableToggle from './TableToggle.vue'

// Tren satu kategori root (+ child) sepanjang rentang (docs/design/design.md §7.12, §8.8). Nominal per periode
// dari backend; rata-rata/tertinggi/terendah hanya ringkasan presentasi.
const store = useReportsStore()
const categories = useCategoriesStore()
const settings = useSettingsStore()
const { t } = useI18n()

const options = computed(() => categories.list.filter((c) =>
  c.parentId === null && c.kind === store.kind && (!c.archived || c.id === store.trendCategoryId)))
const amounts = computed(() => store.trend.map((p) => p.amount))
const labels = computed(() => store.trend.map((p) => formatPeriod(p.period, settings.locale)))
const color = computed(() => (store.kind === 'expense' ? 'var(--expense)' : 'var(--income)'))
const name = computed(() => categories.list.find((c) => c.id === store.trendCategoryId)?.name ?? '')
const series = computed(() => [{ key: 'amount', label: name.value, color: color.value, values: amounts.value }])
const selected = computed(() => store.trend.findIndex((p) => p.period === store.period))
const summary = computed(() => amounts.value.length
  ? [
      { key: 'average', label: t('reports.avgShort'), amount: average(amounts.value)! },
      { key: 'max', label: t('reports.max'), amount: Math.max(...amounts.value) },
      { key: 'min', label: t('reports.min'), amount: Math.min(...amounts.value) },
    ]
  : [])

const { money, num, axis } = useMoneyFormat()
const pick = (v: unknown) => typeof v === 'string' && v && withErrorToast(() => store.setTrendCategory(v))
const select = (i: number) => store.trend[i] && withErrorToast(() => store.select(store.trend[i]!.period))

const row = 'grid grid-cols-[8rem_minmax(7rem,1fr)] items-center gap-4 px-2'
</script>

<template>
  <Card v-if="store.trendCategoryId" class="@container/card gap-4 shadow-xs" data-testid="reports-trend">
    <CardHeader>
      <CardTitle class="text-heading">{{ t('reports.categoryTrend') }}</CardTitle>
      <CardDescription>{{ t('reports.categoryTrendHint', { count: store.range }) }}</CardDescription>
    </CardHeader>
    <CardContent class="flex flex-col gap-3">
      <CategorySelect
        :model-value="store.trendCategoryId" :categories="options" :placeholder="t('categories.pick')"
        class="h-7 w-full @sm/card:w-60" data-testid="trend-category" :aria-label="t('reports.category')" @update:model-value="pick"
      />
      <dl class="grid grid-cols-3 divide-x rounded-lg border text-center" data-part="summary">
        <div v-for="s in summary" :key="s.key" class="flex flex-col gap-0.5 px-2 py-2" :data-part="s.key">
          <dt class="text-meta text-muted-foreground">{{ s.label }}</dt>
          <dd><MoneyText :amount="s.amount" :currency="false" class="text-sm font-semibold" /></dd>
        </div>
      </dl>
      <div data-testid="trend-chart">
        <BarChart
          :labels="labels" :series="series" :label="t('reports.categoryTrendChart', { name })" :format="axis" :value-format="money"
          :selected="selected" @select="select"
        />
      </div>
      <TableToggle testid="trend-table-toggle">
        <TableScroll :label="t('reports.categoryTrend')">
          <div class="flex flex-col" role="table">
            <div :class="cn(row, 'h-8 text-meta text-muted-foreground')" role="row">
              <span role="columnheader">{{ t('reports.period') }}</span>
              <span role="columnheader" class="text-right">{{ name }}</span>
            </div>
            <div v-for="(p, i) in store.trend" :key="p.period" role="row" :class="cn(row, 'h-9', p.period === store.period && 'rounded-lg bg-muted')">
              <span role="cell" class="num">{{ labels[i] }}</span>
              <span role="cell" class="num text-right">{{ num(p.amount) }}</span>
            </div>
          </div>
        </TableScroll>
      </TableToggle>
    </CardContent>
  </Card>
</template>
