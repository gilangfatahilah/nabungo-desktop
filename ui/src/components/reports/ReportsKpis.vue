<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import MoneyText from '@/components/MoneyText.vue'
import { Card, CardAction, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { todayIso } from '@/lib/date'
import { formatIdr } from '@/lib/money'
import { average, perDay, savingsRate } from '@/lib/reportStats'
import { useReportsStore } from '@/stores/reports'
import { useSettingsStore } from '@/stores/settings'
import TrendBadge from './TrendBadge.vue'

// Section card KPI periode fokus (docs/design/design.md §7.11, §8.8). Nominal dari laporan arus kas;
// rata-rata, per hari, dan rasio tabungan hanya pembagian angka backend.
const store = useReportsStore()
const settings = useSettingsStore()
const { t } = useI18n()

const money = (v: number) => formatIdr(v, settings.locale)
const avg = (pick: (p: { income: number; expense: number; net: number }) => number) =>
  average(store.span.map(pick))
const avgText = (v: number | null) => (v === null ? '' : t('reports.average', { count: store.span.length, amount: money(v) }))

// Minus U+2212 seperti nominal uang (lib/money).
const percent = (v: number) => `${v < 0 ? '\u2212' : ''}${Math.abs(v)}%`

const rate = computed(() => (store.focus ? savingsRate(store.focus.income, store.focus.net) : null))
const prevRate = computed(() => (store.previous ? savingsRate(store.previous.income, store.previous.net) : null))

const cards = computed(() => {
  const f = store.focus
  const p = store.previous
  if (!f) return []
  return [
    { key: 'income', label: t('reports.income'), amount: f.income, previous: p?.income, hint: avgText(avg((x) => x.income)) },
    {
      key: 'expense', label: t('reports.expense'), amount: f.expense, previous: p?.expense, upIsBad: true,
      hint: t('reports.perDay', { amount: money(perDay(f.expense, f.from, f.to, todayIso())) }),
    },
    { key: 'net', label: t('reports.remaining'), amount: f.net, previous: p?.net, hint: avgText(avg((x) => x.net)) },
  ]
})
</script>

<template>
  <div v-if="store.focus" class="grid grid-cols-1 gap-4 @xl/main:grid-cols-2 @5xl/main:grid-cols-4" data-testid="reports-kpis">
    <Card
      v-for="c in cards" :key="c.key" class="@container/card gap-4 bg-linear-to-t from-primary/5 to-card shadow-xs"
      :data-testid="`reports-kpi-${c.key}`"
    >
      <CardHeader>
        <CardDescription>{{ c.label }}</CardDescription>
        <CardTitle>
          <MoneyText :amount="c.amount" :kind="c.amount < 0 ? 'expense' : 'neutral'" class="text-title font-semibold" data-part="amount" />
        </CardTitle>
        <CardAction><TrendBadge :amount="c.amount" :previous="c.previous" :up-is-bad="c.upIsBad" /></CardAction>
      </CardHeader>
      <CardFooter class="mt-auto text-meta text-muted-foreground" data-part="hint">{{ c.hint }}</CardFooter>
    </Card>
    <Card class="@container/card gap-4 bg-linear-to-t from-primary/5 to-card shadow-xs" data-testid="reports-kpi-savings">
      <CardHeader>
        <CardDescription>{{ t('reports.savingsRate') }}</CardDescription>
        <CardTitle>
          <span class="num text-title font-semibold" :class="rate !== null && rate < 0 && 'text-expense'" data-part="amount">
            {{ rate === null ? '–' : percent(rate) }}
          </span>
        </CardTitle>
      </CardHeader>
      <CardFooter class="mt-auto text-meta text-muted-foreground" data-part="hint">
        {{ prevRate === null ? t('reports.savingsRateHint') : t('reports.savingsRatePrev', { rate: percent(prevRate) }) }}
      </CardFooter>
    </Card>
  </div>
</template>
