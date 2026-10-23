<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import LineChart from '@/components/charts/LineChart.vue'
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import { formatPeriod } from '@/lib/date'
import { formatCompact } from '@/lib/money'
import { useReportsStore } from '@/stores/reports'
import { useSettingsStore } from '@/stores/settings'

// Chart card arus kas (docs/design/design.md §7.12); rentang memotong data 12 periode dari store reports.
const RANGES = [3, 6, 12] as const

const reports = useReportsStore()
const settings = useSettingsStore()
const { t } = useI18n()
const range = ref<number>(6)

const points = computed(() => reports.cashflow.slice(-range.value))
const labels = computed(() => points.value.map((p) => formatPeriod(p.period, settings.locale)))
const series = computed(() => [
  { key: 'income', label: t('reports.inflow'), color: 'var(--income)', values: points.value.map((p) => p.income) },
  { key: 'expense', label: t('reports.outflow'), color: 'var(--expense)', values: points.value.map((p) => p.expense) },
])
const axis = (v: number) => formatCompact(Math.round(v), settings.locale)

// ToggleGroup single mengirim `undefined` saat item aktif diklik lagi; rentang tetap.
function setRange(v: unknown) {
  if (typeof v === 'string' && v) range.value = Number(v)
}
</script>

<template>
  <Card class="@container/card shadow-xs" data-testid="cashflow-card">
    <CardHeader>
      <CardTitle class="text-heading">{{ t('dashboard.cashflow') }}</CardTitle>
      <CardDescription>{{ t('dashboard.cashflowHint') }}</CardDescription>
      <CardAction>
        <ToggleGroup
          type="single"
          variant="outline"
          size="sm"
          :model-value="String(range)"
          :aria-label="t('dashboard.cashflow')"
          @update:model-value="setRange"
        >
          <ToggleGroupItem v-for="r in RANGES" :key="r" :value="String(r)" class="px-3">
            {{ t('dashboard.periods', { n: r }) }}
          </ToggleGroupItem>
        </ToggleGroup>
      </CardAction>
    </CardHeader>
    <CardContent>
      <LineChart :labels="labels" :series="series" :label="t('dashboard.cashflow')" :format="axis" area />
      <ul class="mt-2 flex gap-4 text-meta text-muted-foreground">
        <li v-for="s in series" :key="s.key" class="flex items-center gap-1.5">
          <span class="size-2 rounded-full" :style="{ background: s.color }" aria-hidden="true" />{{ s.label }}
        </li>
      </ul>
    </CardContent>
  </Card>
</template>
