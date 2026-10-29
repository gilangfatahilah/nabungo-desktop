<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import TableScroll from '@/components/TableScroll.vue'
import BarChart from '@/components/charts/BarChart.vue'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { formatPeriod } from '@/lib/date'
import { formatCompact, formatIdr, moneyParts } from '@/lib/money'
import { withErrorToast } from '@/lib/notify'
import { cn } from '@/lib/utils'
import { useReportsStore } from '@/stores/reports'
import { useSettingsStore } from '@/stores/settings'
import TableToggle from './TableToggle.vue'

// Arus kas per periode dalam rentang (docs/design/design.md §7.12, §8.8): klik batang atau baris tabel
// memindah periode fokus.
const store = useReportsStore()
const settings = useSettingsStore()
const { t } = useI18n()

const num = (v: number) => {
  const p = moneyParts(v, settings.locale)
  return p.sign + p.number
}
const axis = (v: number) => formatCompact(Math.round(v), settings.locale)
const money = (v: number) => formatIdr(v, settings.locale)

const labels = computed(() => store.span.map((p) => formatPeriod(p.period, settings.locale)))
const series = computed(() => [
  { key: 'income', label: t('reports.inflow'), color: 'var(--income)', values: store.span.map((p) => p.income) },
  { key: 'expense', label: t('reports.outflow'), color: 'var(--expense)', values: store.span.map((p) => p.expense) },
])
const selected = computed(() => store.span.findIndex((p) => p.period === store.period))
const select = (period: string) => withErrorToast(() => store.select(period))

const row = 'grid grid-cols-[8rem_repeat(3,minmax(7rem,1fr))] items-center gap-4 px-2'
</script>

<template>
  <Card class="@container/card gap-4 shadow-xs" data-testid="reports-cashflow">
    <CardHeader>
      <CardTitle class="text-heading">{{ t('reports.cashflow') }}</CardTitle>
      <CardDescription>{{ t('reports.cashflowHint') }}</CardDescription>
    </CardHeader>
    <CardContent class="flex flex-col gap-3">
      <div data-testid="cashflow-chart">
        <BarChart
          :labels="labels" :series="series" :label="t('reports.cashflowChart')" :format="axis" :value-format="money"
          :selected="selected" @select="(i) => store.span[i] && select(store.span[i]!.period)"
        />
      </div>
      <ul class="flex gap-4 text-meta text-muted-foreground">
        <li v-for="s in series" :key="s.key" class="flex items-center gap-1.5">
          <span class="size-2 rounded-full" :style="{ background: s.color }" aria-hidden="true" />{{ s.label }}
        </li>
      </ul>
      <TableToggle testid="cashflow-table-toggle">
        <TableScroll :label="t('reports.cashflow')">
          <div class="flex flex-col" role="table">
            <div :class="cn(row, 'h-8 text-meta text-muted-foreground')" role="row">
              <span role="columnheader">{{ t('reports.period') }}</span>
              <span role="columnheader" class="text-right">{{ t('reports.inflow') }}</span>
              <span role="columnheader" class="text-right">{{ t('reports.outflow') }}</span>
              <span role="columnheader" class="text-right">{{ t('reports.net') }}</span>
            </div>
            <div
              v-for="(p, i) in store.span" :key="p.period" role="row" tabindex="0" :data-testid="`cashflow-row-${p.period}`"
              :aria-selected="p.period === store.period"
              :class="cn(row, 'h-9 cursor-pointer rounded-lg outline-none hover:bg-muted/50 focus-visible:ring-2 focus-visible:ring-ring', p.period === store.period && 'bg-muted')"
              @click="select(p.period)" @keydown.enter.prevent="select(p.period)" @keydown.space.prevent="select(p.period)"
            >
              <span role="cell" class="num">{{ labels[i] }}</span>
              <span role="cell" class="num text-right">{{ num(p.income) }}</span>
              <span role="cell" class="num text-right">{{ num(p.expense) }}</span>
              <span role="cell" data-part="net" :class="cn('num text-right', p.net < 0 && 'text-destructive')">{{ num(p.net) }}</span>
            </div>
          </div>
        </TableScroll>
      </TableToggle>
    </CardContent>
  </Card>
</template>
