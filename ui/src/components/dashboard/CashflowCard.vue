<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import LineChart from '@/components/charts/LineChart.vue'
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { formatPeriod } from '@/lib/date'
import { formatCompact, formatIdr } from '@/lib/money'
import { useReportsStore } from '@/stores/reports'
import { useSettingsStore } from '@/stores/settings'

// Chart card tren (docs/design/design.md §7.12, §8.1): tab Arus kas | Kekayaan bersih; dropdown rentang
// memotong data 12 periode dari store reports (label "bulan" = periode budget).
const RANGES = [3, 6, 12] as const
const VIEWS = ['cashflow', 'netWorth'] as const
type View = (typeof VIEWS)[number]

const reports = useReportsStore()
const settings = useSettingsStore()
const { t } = useI18n()
const range = ref<number>(6)
const view = ref<View>('cashflow')

const cashflow = computed(() => reports.cashflow.slice(-range.value))
const netWorth = computed(() => reports.netWorth.slice(-range.value))
const labels = computed(() =>
  (view.value === 'cashflow' ? cashflow.value : netWorth.value).map((p) => formatPeriod(p.period, settings.locale)))
const series = computed(() => view.value === 'cashflow'
  ? [
      { key: 'income', label: t('reports.inflow'), color: 'var(--income)', values: cashflow.value.map((p) => p.income) },
      { key: 'expense', label: t('reports.outflow'), color: 'var(--expense)', values: cashflow.value.map((p) => p.expense) },
    ]
  : [{ key: 'net', label: t('dashboard.netWorth'), color: 'var(--chart-1)', values: netWorth.value.map((p) => p.net) }])
const axis = (v: number) => formatCompact(Math.round(v), settings.locale)
const money = (v: number) => formatIdr(v, settings.locale)

function setRange(v: unknown) {
  if (typeof v === 'string' && v) range.value = Number(v)
}
</script>

<template>
  <Card class="@container/card shadow-xs" data-testid="cashflow-card">
    <CardHeader>
      <CardTitle class="text-heading">{{ t('dashboard.trend') }}</CardTitle>
      <CardDescription>{{ t(view === 'cashflow' ? 'dashboard.cashflowHint' : 'dashboard.netWorthTrendHint') }}</CardDescription>
      <CardAction>
        <Select :model-value="String(range)" @update:model-value="setRange">
          <SelectTrigger size="sm" class="w-40" data-testid="trend-range" :aria-label="t('dashboard.trend')"><SelectValue /></SelectTrigger>
          <SelectContent align="end">
            <SelectItem v-for="r in RANGES" :key="r" :value="String(r)">{{ t(`dashboard.ranges.${r}`) }}</SelectItem>
          </SelectContent>
        </Select>
      </CardAction>
    </CardHeader>
    <CardContent class="flex flex-col gap-3">
      <Tabs v-model="view">
        <TabsList variant="line">
          <TabsTrigger v-for="v in VIEWS" :key="v" :value="v" :data-testid="`trend-tab-${v}`">
            {{ t(v === 'cashflow' ? 'dashboard.cashflow' : 'dashboard.netWorth') }}
          </TabsTrigger>
        </TabsList>
      </Tabs>
      <LineChart :labels="labels" :series="series" :label="t(view === 'cashflow' ? 'dashboard.cashflow' : 'dashboard.netWorth')" :format="axis" :value-format="money" area />
      <ul class="flex gap-4 text-meta text-muted-foreground">
        <li v-for="s in series" :key="s.key" class="flex items-center gap-1.5">
          <span class="size-2 rounded-full" :style="{ background: s.color }" aria-hidden="true" />{{ s.label }}
        </li>
      </ul>
    </CardContent>
  </Card>
</template>
