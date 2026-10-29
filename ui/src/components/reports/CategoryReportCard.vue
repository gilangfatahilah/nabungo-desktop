<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { ArrowUpRight, ChevronRight } from '@lucide/vue'
import EmptyState from '@/components/EmptyState.vue'
import MoneyText from '@/components/MoneyText.vue'
import DonutChart from '@/components/charts/DonutChart.vue'
import { Button } from '@/components/ui/button'
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import type { CategoryKindDto } from '@/ipc'
import { colorClass } from '@/lib/colors'
import { formatPeriod } from '@/lib/date'
import { iconFor } from '@/lib/icons'
import { formatCompact, formatIdr } from '@/lib/money'
import { withErrorToast } from '@/lib/notify'
import { formatBp } from '@/lib/percent'
import { cn } from '@/lib/utils'
import { useCategoriesStore } from '@/stores/categories'
import { useReportsStore } from '@/stores/reports'
import { useSettingsStore } from '@/stores/settings'
import { EMPTY_FILTER, useTransactionsStore } from '@/stores/transactions'
import TrendBadge from './TrendBadge.vue'

// Kategori periode fokus (docs/design/design.md §8.8): donut 5 root teratas + Lainnya, baris root dengan
// porsi dan tren vs periode lalu, child bisa dibuka. Klik baris = pilih untuk Tren kategori;
// panah = Transaksi terfilter kategori (termasuk child) dan rentang periode.
const KINDS: CategoryKindDto[] = ['expense', 'income']
const TOP = 5
const PALETTE = ['var(--chart-1)', 'var(--chart-2)', 'var(--chart-3)', 'var(--chart-4)', 'var(--chart-5)']
const OTHER_COLOR = 'var(--muted-foreground)'

const store = useReportsStore()
const categories = useCategoriesStore()
const settings = useSettingsStore()
const txns = useTransactionsStore()
const router = useRouter()
const { t } = useI18n()

const byId = computed(() => new Map(categories.list.map((c) => [c.id, c])))
const name = (id: string) => byId.value.get(id)?.name ?? ''
const money = (v: number) => formatIdr(v, settings.locale)
const upIsBad = computed(() => store.kind === 'expense')

const active = computed(() => store.comparison.filter((r) => r.amount > 0))
const total = computed(() => active.value.reduce((s, r) => s + r.amount, 0))
// Porsi dalam basis point; hanya untuk tampilan, nominalnya tetap dari backend.
const shareBp = (amount: number) => (total.value > 0 ? Math.round((amount * 10000) / total.value) : 0)
const segments = computed(() => {
  const top = active.value.slice(0, TOP).map((r, i) => ({ key: r.categoryId, label: name(r.categoryId), value: r.amount, color: PALETTE[i]! }))
  const other = active.value.slice(TOP).reduce((s, r) => s + r.amount, 0)
  return other > 0 ? [...top, { key: 'other', label: t('dashboard.other'), value: other, color: OTHER_COLOR }] : top
})
const colorOf = computed(() => new Map(segments.value.map((s) => [s.key, s.color])))
const hovered = ref<string | null>(null)

const open = ref(new Set<string>())
function toggle(id: string) {
  const next = new Set(open.value)
  if (!next.delete(id)) next.add(id)
  open.value = next
}

const setKind = (k: unknown) => withErrorToast(() => store.setKind(k as CategoryKindDto))
const pick = (id: string) => withErrorToast(() => store.setTrendCategory(id))

async function viewTxns(categoryId: string) {
  const f = store.focus
  if (!f) return
  await withErrorToast(() => txns.setQuery({ ...EMPTY_FILTER, categoryId, dateFrom: f.from, dateTo: f.to }))
  await router.push('/transactions')
}
</script>

<template>
  <Card class="@container/card gap-4 shadow-xs" data-testid="reports-categories">
    <CardHeader>
      <CardTitle class="text-heading" data-testid="breakdown-title">
        {{ t('reports.categoriesTitle', { period: store.period ? formatPeriod(store.period, settings.locale) : '' }) }}
      </CardTitle>
      <CardDescription>{{ t('reports.categoriesHint') }}</CardDescription>
      <CardAction>
        <Tabs :model-value="store.kind" @update:model-value="setKind">
          <TabsList>
            <TabsTrigger v-for="k in KINDS" :key="k" :value="k" class="px-3" :data-testid="`kind-${k}`">{{ t(`reports.${k}`) }}</TabsTrigger>
          </TabsList>
        </Tabs>
      </CardAction>
    </CardHeader>
    <CardContent class="flex flex-col gap-4">
      <EmptyState
        v-if="!store.comparison.length" data-testid="breakdown-empty" class="py-6"
        :title="store.kind === 'income' ? t('reports.emptyIncome') : t('reports.emptyExpense')"
      />
      <template v-else>
        <div v-if="segments.length" class="relative mx-auto size-36">
          <DonutChart v-model:active="hovered" :segments="segments" :label="t('reports.categoriesDonut')" :format="money" :locale="settings.locale" />
          <div class="pointer-events-none absolute inset-0 flex flex-col items-center justify-center">
            <span class="text-meta text-muted-foreground">IDR</span>
            <span class="num text-heading font-semibold" data-part="donut-total">{{ formatCompact(total, settings.locale) }}</span>
          </div>
        </div>
        <ul class="flex flex-col" :aria-label="t('reports.categoriesTitle', { period: store.period ? formatPeriod(store.period, settings.locale) : '' })">
          <li
            v-for="r in store.comparison" :key="r.categoryId" data-testid="breakdown-row" :data-category="r.categoryId"
            class="flex flex-col" @mouseenter="hovered = colorOf.has(r.categoryId) ? r.categoryId : 'other'" @mouseleave="hovered = null"
          >
            <div :class="cn('flex items-center gap-1 rounded-lg pr-1 hover:bg-muted/50', store.trendCategoryId === r.categoryId && 'bg-muted')">
              <Button
                v-if="r.children.length" variant="ghost" size="icon-xs" class="shrink-0" data-part="expand"
                :aria-expanded="open.has(r.categoryId)" :aria-label="t('reports.showChildren', { name: name(r.categoryId) })"
                @click="toggle(r.categoryId)"
              >
                <ChevronRight :class="['transition-transform', open.has(r.categoryId) && 'rotate-90']" aria-hidden="true" />
              </Button>
              <span v-else class="size-6 shrink-0" aria-hidden="true" />
              <button
                type="button" data-part="select" :aria-pressed="store.trendCategoryId === r.categoryId"
                class="flex min-h-12 min-w-0 flex-1 items-center gap-3 rounded-md py-1.5 text-left outline-none focus-visible:ring-2 focus-visible:ring-ring"
                @click="pick(r.categoryId)"
              >
                <span :class="cn('relative flex size-7 shrink-0 items-center justify-center rounded-md', colorClass(byId.get(r.categoryId)?.color ?? null))">
                  <component :is="iconFor(byId.get(r.categoryId)?.icon ?? null)" class="size-4" aria-hidden="true" />
                </span>
                <span class="flex min-w-0 flex-1 flex-col">
                  <span data-part="name" class="truncate" :title="name(r.categoryId)">{{ name(r.categoryId) }}</span>
                  <span class="flex items-center gap-1.5 text-meta text-muted-foreground">
                    <span v-if="r.amount > 0" class="size-2 shrink-0 rounded-full" :style="{ background: colorOf.get(r.categoryId) ?? OTHER_COLOR }" aria-hidden="true" />
                    <span class="num" data-part="share">{{ formatBp(shareBp(r.amount), settings.locale) }}</span>
                    <span class="truncate">· {{ t('reports.previousAmount', { amount: money(r.previous) }) }}</span>
                  </span>
                </span>
                <span class="flex shrink-0 flex-col items-end gap-1">
                  <MoneyText :amount="r.amount" :currency="false" class="font-medium" data-part="amount" />
                  <TrendBadge :amount="r.amount" :previous="r.previous" :up-is-bad="upIsBad" />
                </span>
              </button>
              <Button
                variant="ghost" size="icon-sm" class="shrink-0" data-part="view-txns"
                :aria-label="t('reports.viewTransactions', { name: name(r.categoryId) })" :title="t('reports.viewTransactions', { name: name(r.categoryId) })"
                @click="viewTxns(r.categoryId)"
              >
                <ArrowUpRight aria-hidden="true" />
              </Button>
            </div>
            <ul v-if="open.has(r.categoryId)" class="ml-10 flex flex-col border-l pl-3" data-part="children">
              <li v-for="c in r.children" :key="c.categoryId" class="flex min-h-9 items-center gap-2 text-sm" data-testid="breakdown-child">
                <span class="min-w-0 flex-1 truncate">
                  {{ c.categoryId === r.categoryId ? t('reports.directInCategory') : name(c.categoryId) }}
                </span>
                <TrendBadge :amount="c.amount" :previous="c.previous" :up-is-bad="upIsBad" />
                <MoneyText :amount="c.amount" :currency="false" class="w-28 shrink-0 text-right" />
                <Button
                  v-if="c.categoryId !== r.categoryId" variant="ghost" size="icon-xs" class="shrink-0"
                  :aria-label="t('reports.viewTransactions', { name: name(c.categoryId) })" :title="t('reports.viewTransactions', { name: name(c.categoryId) })"
                  @click="viewTxns(c.categoryId)"
                >
                  <ArrowUpRight aria-hidden="true" />
                </Button>
                <span v-else class="size-6 shrink-0" aria-hidden="true" />
              </li>
            </ul>
          </li>
        </ul>
      </template>
    </CardContent>
  </Card>
</template>
