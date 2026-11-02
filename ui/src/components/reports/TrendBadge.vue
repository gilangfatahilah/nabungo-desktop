<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Minus, TrendingDown, TrendingUp } from '@lucide/vue'
import { Badge } from '@/components/ui/badge'
import { formatTrend, trendPercent } from '@/lib/trend'

// Badge tren vs periode lalu (docs/design/design.md §7.11): warna ikon mengikuti makna, bukan arah.
// Tanpa pembanding (periode lalu 0) tidak tampil.
const props = withDefaults(defineProps<{ amount: number; previous: number | undefined; upIsBad?: boolean }>(), { upIsBad: false })
const { t } = useI18n()

const trend = computed(() => {
  const percent = trendPercent(props.amount, props.previous)
  if (percent === null) return null
  const up = percent > 0
  return {
    text: formatTrend(percent),
    icon: percent === 0 ? Minus : up ? TrendingUp : TrendingDown,
    tone: percent === 0 ? 'text-muted-foreground' : up !== props.upIsBad ? 'text-income' : 'text-expense',
    label: t(percent === 0 ? 'dashboard.trendFlat' : up ? 'dashboard.trendUp' : 'dashboard.trendDown'),
  }
})
</script>

<template>
  <Badge v-if="trend" variant="outline" data-part="trend" :title="trend.label">
    <component :is="trend.icon" :class="trend.tone" aria-hidden="true" />
    <span class="num">{{ trend.text }}</span>
    <span class="sr-only">{{ trend.label }}</span>
  </Badge>
</template>
