<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import TableScroll from '@/components/TableScroll.vue'
import LineChart from '@/components/charts/LineChart.vue'
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { formatPeriod } from '@/lib/date'
import { formatCompact, formatIdr, moneyParts } from '@/lib/money'
import { cn } from '@/lib/utils'
import { useReportsStore } from '@/stores/reports'
import { useSettingsStore } from '@/stores/settings'
import TableToggle from './TableToggle.vue'
import TrendBadge from './TrendBadge.vue'

// Kekayaan bersih per akhir periode dalam rentang (docs/design/design.md §7.12, §8.8); badge = awal → akhir rentang.
const store = useReportsStore()
const settings = useSettingsStore()
const { t } = useI18n()

const num = (v: number) => {
  const p = moneyParts(v, settings.locale)
  return p.sign + p.number
}
const axis = (v: number) => formatCompact(Math.round(v), settings.locale)
const money = (v: number) => formatIdr(v, settings.locale)

const points = computed(() => store.rangeNetWorth)
const labels = computed(() => points.value.map((p) => formatPeriod(p.period, settings.locale)))
const series = computed(() => [
  { key: 'net', label: t('reports.net'), color: 'var(--chart-1)', values: points.value.map((p) => p.net) },
  { key: 'assets', label: t('reports.assets'), color: 'var(--chart-2)', values: points.value.map((p) => p.assets), dashed: true },
  { key: 'liabilities', label: t('reports.liabilities'), color: 'var(--chart-3)', values: points.value.map((p) => p.liabilities), dashed: true },
])

const row = 'grid grid-cols-[8rem_repeat(3,minmax(7rem,1fr))] items-center gap-4 px-2'
</script>

<template>
  <Card class="@container/card gap-4 shadow-xs" data-testid="reports-networth">
    <CardHeader>
      <CardTitle class="text-heading">{{ t('reports.netWorth') }}</CardTitle>
      <CardDescription>{{ t('reports.netWorthHint') }}</CardDescription>
      <CardAction v-if="points.length > 1">
        <TrendBadge :amount="points.at(-1)!.net" :previous="points[0]!.net" />
      </CardAction>
    </CardHeader>
    <CardContent class="flex flex-col gap-3">
      <div data-testid="networth-chart">
        <LineChart :labels="labels" :series="series" :label="t('reports.netWorthChart')" :format="axis" :value-format="money" area smooth />
      </div>
      <ul class="flex gap-4 text-meta text-muted-foreground">
        <li v-for="s in series" :key="s.key" class="flex items-center gap-1.5">
          <span class="size-2 rounded-full" :style="{ background: s.color }" aria-hidden="true" />{{ s.label }}
        </li>
      </ul>
      <TableToggle testid="networth-table-toggle">
        <TableScroll :label="t('reports.netWorth')">
          <div class="flex flex-col" role="table">
            <div :class="cn(row, 'h-8 text-meta text-muted-foreground')" role="row">
              <span role="columnheader">{{ t('reports.period') }}</span>
              <span role="columnheader" class="text-right">{{ t('reports.assets') }}</span>
              <span role="columnheader" class="text-right">{{ t('reports.liabilities') }}</span>
              <span role="columnheader" class="text-right">{{ t('reports.net') }}</span>
            </div>
            <div v-for="(p, i) in points" :key="p.period" role="row" :data-testid="`networth-row-${p.period}`" :class="cn(row, 'h-9')">
              <span role="cell" class="num">{{ labels[i] }}</span>
              <span role="cell" class="num text-right">{{ num(p.assets) }}</span>
              <span role="cell" class="num text-right">{{ num(p.liabilities) }}</span>
              <span role="cell" :class="cn('num text-right', p.net < 0 && 'text-destructive')">{{ num(p.net) }}</span>
            </div>
          </div>
        </TableScroll>
      </TableToggle>
    </CardContent>
  </Card>
</template>
