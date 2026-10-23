<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Minus, TrendingDown, TrendingUp } from '@lucide/vue'
import MoneyText from '@/components/MoneyText.vue'
import { Badge } from '@/components/ui/badge'
import { Card, CardAction, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { formatRange } from '@/lib/date'
import { formatTrend, trendPercent } from '@/lib/trend'
import { useReportsStore } from '@/stores/reports'
import { useSettingsStore } from '@/stores/settings'

// KPI Dasbor (docs/design/design.md §7.11): periode berjalan = titik terakhir laporan.
const reports = useReportsStore()
const settings = useSettingsStore()
const { t } = useI18n()

interface Kpi {
  key: string
  label: string
  amount: number
  previous: number | undefined
  /** Naik itu buruk (pengeluaran): warna ikon tren dibalik. */
  upIsBad?: boolean
  hint: string
}

const cards = computed<Kpi[]>(() => {
  const cf = reports.cashflow
  const cur = cf.at(-1)
  const prev = cf.at(-2)
  const nw = reports.netWorth
  const range = cur ? t('dashboard.period', { range: formatRange(cur.from, cur.to, settings.locale) }) : ''
  return [
    { key: 'remaining', label: t('dashboard.remaining'), amount: cur?.net ?? 0, previous: prev?.net, hint: range },
    { key: 'income', label: t('dashboard.income'), amount: cur?.income ?? 0, previous: prev?.income, hint: range },
    { key: 'expense', label: t('dashboard.expense'), amount: cur?.expense ?? 0, previous: prev?.expense, upIsBad: true, hint: range },
    { key: 'net-worth', label: t('dashboard.netWorth'), amount: nw.at(-1)?.net ?? 0, previous: nw.at(-2)?.net, hint: t('dashboard.netWorthHint') },
  ]
})

function trend(k: Kpi) {
  const percent = trendPercent(k.amount, k.previous)
  if (percent === null) return null
  const up = percent > 0
  const tone = percent === 0 ? 'text-muted-foreground' : up !== !!k.upIsBad ? 'text-income' : 'text-expense'
  return {
    text: formatTrend(percent),
    icon: percent === 0 ? Minus : up ? TrendingUp : TrendingDown,
    tone,
    label: t(percent === 0 ? 'dashboard.trendFlat' : up ? 'dashboard.trendUp' : 'dashboard.trendDown'),
  }
}
</script>

<template>
  <div class="grid grid-cols-1 gap-4 @xl/main:grid-cols-2 @5xl/main:grid-cols-4">
    <Card
      v-for="k in cards"
      :key="k.key"
      class="@container/card bg-linear-to-t from-primary/5 to-card shadow-xs"
      :data-testid="`kpi-${k.key}`"
    >
      <CardHeader>
        <CardDescription>{{ k.label }}</CardDescription>
        <CardTitle class="text-title font-semibold">
          <MoneyText :amount="k.amount" :kind="k.amount < 0 ? 'expense' : 'neutral'" data-part="amount" />
        </CardTitle>
        <CardAction v-if="trend(k)">
          <Badge variant="outline" data-part="trend">
            <component :is="trend(k)!.icon" :class="trend(k)!.tone" aria-hidden="true" />
            <span class="num">{{ trend(k)!.text }}</span>
          </Badge>
        </CardAction>
      </CardHeader>
      <CardFooter class="flex-col items-start gap-1">
        <div v-if="trend(k)" class="flex items-center gap-2 font-medium">
          {{ trend(k)!.label }}
          <component :is="trend(k)!.icon" class="size-4" :class="trend(k)!.tone" aria-hidden="true" />
        </div>
        <div class="text-meta text-muted-foreground">{{ k.hint }}</div>
      </CardFooter>
    </Card>
  </div>
</template>
