<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Minus, TrendingDown, TrendingUp } from '@lucide/vue'
import AmountsToggle from '@/components/AmountsToggle.vue'
import MoneyText from '@/components/MoneyText.vue'
import { Badge } from '@/components/ui/badge'
import { formatTrend, trendPercent } from '@/lib/trend'
import { useReportsStore } from '@/stores/reports'

// Hero Dasbor (docs/design/design.md §8.1): kekayaan bersih + tiga statistik periode berjalan
// (titik terakhir laporan); tren dibanding titik sebelumnya.
const reports = useReportsStore()
const { t } = useI18n()

interface Kpi {
  key: string
  label: string
  amount: number
  previous: number | undefined
  /** Naik itu buruk (pengeluaran): warna ikon tren dibalik. */
  upIsBad?: boolean
}

const netWorth = computed<Kpi>(() => ({
  key: 'net-worth', label: t('dashboard.netWorth'),
  amount: reports.netWorth.at(-1)?.net ?? 0, previous: reports.netWorth.at(-2)?.net,
}))
const stats = computed<Kpi[]>(() => {
  const cur = reports.cashflow.at(-1)
  const prev = reports.cashflow.at(-2)
  return [
    { key: 'remaining', label: t('dashboard.remaining'), amount: cur?.net ?? 0, previous: prev?.net },
    { key: 'income', label: t('dashboard.income'), amount: cur?.income ?? 0, previous: prev?.income },
    { key: 'expense', label: t('dashboard.expense'), amount: cur?.expense ?? 0, previous: prev?.expense, upIsBad: true },
  ]
})

function trend(k: Kpi) {
  const percent = trendPercent(k.amount, k.previous)
  if (percent === null) return null
  const up = percent > 0
  return {
    text: formatTrend(percent),
    icon: percent === 0 ? Minus : up ? TrendingUp : TrendingDown,
    tone: percent === 0 ? 'text-muted-foreground' : up !== !!k.upIsBad ? 'text-income' : 'text-expense',
    label: t(percent === 0 ? 'dashboard.trendFlat' : up ? 'dashboard.trendUp' : 'dashboard.trendDown'),
  }
}
</script>

<template>
  <section class="flex flex-col gap-6 @3xl/main:flex-row @3xl/main:items-end @3xl/main:justify-between" data-testid="dashboard-hero">
    <div class="flex flex-col gap-1" :data-testid="`kpi-${netWorth.key}`">
      <div class="flex items-center gap-1">
        <span class="text-meta text-muted-foreground">{{ netWorth.label }}</span>
        <AmountsToggle size="icon-sm" class="text-muted-foreground" />
      </div>
      <div class="flex flex-wrap items-center gap-3">
        <MoneyText
          :amount="netWorth.amount" :kind="netWorth.amount < 0 ? 'expense' : 'neutral'"
          class="text-display font-semibold" data-part="amount"
        />
        <Badge v-if="trend(netWorth)" variant="outline" data-part="trend" :title="trend(netWorth)!.label">
          <component :is="trend(netWorth)!.icon" :class="trend(netWorth)!.tone" aria-hidden="true" />
          <span class="num">{{ trend(netWorth)!.text }}</span>
        </Badge>
      </div>
      <span class="text-sm text-muted-foreground">{{ $t('dashboard.netWorthHint') }}</span>
    </div>
    <dl class="grid grid-cols-1 gap-4 @xl/main:grid-cols-3 @xl/main:gap-0 @xl/main:divide-x">
      <div v-for="k in stats" :key="k.key" class="flex flex-col gap-1 @xl/main:px-6 @xl/main:first:pl-0 @xl/main:last:pr-0" :data-testid="`kpi-${k.key}`">
        <dt class="text-meta text-muted-foreground">{{ k.label }}</dt>
        <dd class="flex items-center gap-2">
          <MoneyText :amount="k.amount" :kind="k.amount < 0 ? 'expense' : 'neutral'" class="text-heading font-semibold" data-part="amount" />
          <Badge v-if="trend(k)" variant="outline" data-part="trend" :title="trend(k)!.label">
            <component :is="trend(k)!.icon" :class="trend(k)!.tone" aria-hidden="true" />
            <span class="num">{{ trend(k)!.text }}</span>
          </Badge>
        </dd>
      </div>
    </dl>
  </section>
</template>
