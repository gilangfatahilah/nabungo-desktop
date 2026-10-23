<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import MoneyText from '@/components/MoneyText.vue'
import DonutChart from '@/components/charts/DonutChart.vue'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import type { BudgetTotalDto, CategoryAmountDto } from '@/ipc'
import { formatIdr } from '@/lib/money'
import { formatBp } from '@/lib/percent'
import { TOP_LIMIT } from '@/stores/dashboard'
import { useCategoriesStore } from '@/stores/categories'
import { useReportsStore } from '@/stores/reports'
import { useSettingsStore } from '@/stores/settings'

// Pengeluaran periode berjalan: periode ini vs lalu (laporan arus kas), sisa budget, donut 5 kategori root
// teratas + "Lainnya" (docs/design/design.md §8.1). Urutan & nominal dari backend; persen hanya presentasi.
const props = defineProps<{ rows: CategoryAmountDto[]; budget: BudgetTotalDto | null }>()
const PALETTE = ['var(--chart-1)', 'var(--chart-2)', 'var(--chart-3)', 'var(--chart-4)', 'var(--chart-5)']
const OTHER_COLOR = 'var(--muted-foreground)'

const { t } = useI18n()
const categories = useCategoriesStore()
const reports = useReportsStore()
const settings = useSettingsStore()

const byId = computed(() => new Map(categories.list.map((c) => [c.id, c])))
const current = computed(() => reports.cashflow.at(-1)?.expense ?? 0)
const previous = computed(() => reports.cashflow.at(-2)?.expense ?? 0)
const total = computed(() => props.rows.reduce((s, r) => s + r.amount, 0))
const top = computed(() => props.rows.slice(0, TOP_LIMIT).map((r, i) => ({
  key: r.categoryId, label: byId.value.get(r.categoryId)?.name ?? '', value: r.amount, color: PALETTE[i]!,
})))
const other = computed(() => props.rows.slice(TOP_LIMIT).reduce((s, r) => s + r.amount, 0))
const segments = computed(() => other.value > 0
  ? [...top.value, { key: 'other', label: t('dashboard.other'), value: other.value, color: OTHER_COLOR }]
  : top.value)
// Segmen yang disorot; hover legenda dan donut berbagi state ini.
const hovered = ref<string | null>(null)
const money = (v: number) => formatIdr(v, settings.locale)
const share = (amount: number) => (total.value > 0 ? Math.round((amount * 10000) / total.value) : 0)
</script>

<template>
  <Card v-if="rows.length" class="@container/card gap-4 shadow-xs" data-testid="dashboard-expense">
    <CardHeader>
      <CardTitle class="text-heading">
        <RouterLink to="/reports" class="hover:underline">{{ $t('dashboard.expenseTitle') }}</RouterLink>
      </CardTitle>
    </CardHeader>
    <CardContent class="flex flex-col gap-4">
      <dl class="grid grid-cols-3 divide-x rounded-lg border text-center">
        <div class="flex flex-col gap-0.5 px-2 py-2">
          <dt class="text-meta text-muted-foreground">{{ $t('dashboard.thisPeriod') }}</dt>
          <dd><MoneyText :amount="current" :currency="false" class="text-sm font-semibold" data-part="current" /></dd>
        </div>
        <div class="flex flex-col gap-0.5 px-2 py-2">
          <dt class="text-meta text-muted-foreground">{{ $t('dashboard.lastPeriod') }}</dt>
          <dd><MoneyText :amount="previous" :currency="false" class="text-sm font-semibold" data-part="previous" /></dd>
        </div>
        <div class="flex flex-col gap-0.5 px-2 py-2">
          <dt class="text-meta text-muted-foreground">{{ $t('dashboard.budgetLeft') }}</dt>
          <dd>
            <MoneyText
              v-if="budget && budget.available > 0" :amount="budget.remaining" :currency="false"
              :kind="budget.remaining < 0 ? 'expense' : 'neutral'" class="text-sm font-semibold"
            />
            <span v-else class="text-sm text-muted-foreground">–</span>
          </dd>
        </div>
      </dl>
      <div class="flex flex-col items-center gap-4 @sm/card:flex-row">
        <div class="relative size-32 shrink-0">
          <DonutChart v-model:active="hovered" :segments="segments" :label="$t('dashboard.expenseDonut')" :format="money" :locale="settings.locale" />
          <div v-if="top[0]" class="pointer-events-none absolute inset-0 flex flex-col items-center justify-center">
            <span class="num text-heading font-semibold">{{ formatBp(share(top[0].value), settings.locale) }}</span>
            <span class="max-w-16 truncate text-meta text-muted-foreground">{{ top[0].label }}</span>
          </div>
        </div>
        <ul class="flex w-full min-w-0 flex-col gap-1.5 text-sm">
          <li
            v-for="s in top" :key="s.key" class="flex items-center gap-2 transition-opacity" data-testid="top-category-row"
            :class="{ 'opacity-50': hovered !== null && hovered !== s.key }" @mouseenter="hovered = s.key" @mouseleave="hovered = null"
          >
            <span class="size-2.5 shrink-0 rounded-full" :style="{ background: s.color }" aria-hidden="true" />
            <span class="truncate">{{ s.label }}</span>
            <MoneyText :amount="s.value" :currency="false" class="ml-auto shrink-0 font-medium" />
          </li>
          <li
            v-if="other > 0" class="flex items-center gap-2 text-muted-foreground transition-opacity" data-testid="top-category-other"
            :class="{ 'opacity-50': hovered !== null && hovered !== 'other' }" @mouseenter="hovered = 'other'" @mouseleave="hovered = null"
          >
            <span class="size-2.5 shrink-0 rounded-full" :style="{ background: OTHER_COLOR }" aria-hidden="true" />
            <span class="truncate">{{ $t('dashboard.other') }}</span>
            <MoneyText :amount="other" :currency="false" class="ml-auto shrink-0" />
          </li>
        </ul>
      </div>
    </CardContent>
  </Card>
</template>
