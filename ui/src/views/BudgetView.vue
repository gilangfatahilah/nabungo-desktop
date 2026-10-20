<script setup lang="ts">
import { ChevronLeft, ChevronRight, Copy } from '@lucide/vue'
import { computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from 'vue-sonner'
import BudgetBar from '@/components/BudgetBar.vue'
import EmptyState from '@/components/EmptyState.vue'
import PageHeader from '@/components/PageHeader.vue'
import BudgetAmountCell from '@/components/budget/BudgetAmountCell.vue'
import { Button } from '@/components/ui/button'
import { formatPeriod, formatRange } from '@/lib/date'
import { moneyParts } from '@/lib/money'
import { withErrorToast } from '@/lib/notify'
import { cn } from '@/lib/utils'
import { useBudgetStore } from '@/stores/budget'
import { useCategoriesStore } from '@/stores/categories'
import { useSettingsStore } from '@/stores/settings'
import { useTransactionsStore } from '@/stores/transactions'

const store = useBudgetStore()
const categories = useCategoriesStore()
const settings = useSettingsStore()
const { t } = useI18n()

const names = computed(() => new Map(categories.list.map((c) => [c.id, c.name])))
const o = computed(() => store.overview)
// Kolom: kategori | budget | terpakai | sisa | bar + persen.
const grid = 'grid grid-cols-[minmax(0,1fr)_8rem_8rem_8rem_10rem] items-center gap-4 px-2'

onMounted(() => {
  void withErrorToast(async () => {
    if (!categories.loaded) await categories.load()
    await store.init()
  })
})

const txns = useTransactionsStore()
watch(() => txns.ledgerVersion, () => {
  if (store.period) void withErrorToast(() => store.load(store.period!))
})

const num = (v: number) => {
  const p = moneyParts(v, settings.locale)
  return p.sign + p.number
}
const percent = (spent: number, budget: number) => (budget > 0 ? Math.round((Math.max(spent, 0) * 100) / budget) : null)

function save(categoryId: string, amount: number | null) {
  void withErrorToast(() => store.setAmount(categoryId, amount))
}

function copy() {
  void withErrorToast(async () => {
    const n = await store.copyPrevious()
    toast(n > 0 ? t('budget.copied', { n }) : t('budget.copiedNone'))
  })
}
</script>

<template>
  <PageHeader :title="t('nav.budget')">
    <template v-if="o">{{ formatRange(o.from, o.to, settings.locale) }}</template>
    <template #actions>
      <Button variant="ghost" size="icon" :aria-label="t('budget.prev')" @click="withErrorToast(() => store.go('prev'))">
        <ChevronLeft />
      </Button>
      <span data-testid="budget-period" class="num min-w-20 text-center font-medium">{{ o ? formatPeriod(o.period, settings.locale) : '' }}</span>
      <Button variant="ghost" size="icon" :aria-label="t('budget.next')" @click="withErrorToast(() => store.go('next'))">
        <ChevronRight />
      </Button>
      <Button variant="outline" @click="copy"><Copy /> {{ t('budget.copy') }}</Button>
    </template>
  </PageHeader>

  <EmptyState v-if="o && !o.rows.length" :title="t('budget.empty')" :description="t('budget.emptyHint')" />
  <div v-else-if="o" class="-mx-2 flex flex-col" role="table">
    <div :class="cn(grid, 'h-8 text-meta text-muted-foreground')" role="row">
      <span role="columnheader">{{ t('budget.category') }}</span>
      <span role="columnheader" class="text-right">{{ t('budget.amount') }}</span>
      <span role="columnheader" class="text-right">{{ t('budget.spent') }}</span>
      <span role="columnheader" class="text-right">{{ t('budget.remaining') }}</span>
      <span role="columnheader" />
    </div>
    <div
      v-for="r in o.rows" :key="r.categoryId" role="row" :data-testid="`budget-row-${r.categoryId}`"
      :class="cn(grid, 'rounded-lg hover:bg-muted/50', r.parentId ? 'h-10' : 'h-12')"
    >
      <span :class="cn('truncate', r.parentId ? 'pl-6' : 'font-medium')">{{ names.get(r.categoryId) ?? '' }}</span>
      <BudgetAmountCell
        :category-id="r.categoryId" :name="names.get(r.categoryId) ?? ''" :amount="r.budget" :locale="settings.locale"
        @save="save(r.categoryId, $event)" @error="toast.error($event)"
      />
      <span class="num text-right">{{ num(r.spent) }}</span>
      <span
        :data-testid="`budget-remaining-${r.categoryId}`"
        :class="cn('num text-right', r.remaining === null && 'text-muted-foreground', r.remaining !== null && r.remaining < 0 && 'text-destructive')"
      >{{ r.remaining === null ? '—' : num(r.remaining) }}</span>
      <span class="flex items-center gap-3">
        <template v-if="r.budget !== null">
          <!-- Ruang kanan untuk bagian lebih yang menjulur keluar track (maks 25% lebar track). -->
          <span class="flex-1 pr-[20%]"><BudgetBar :spent="r.spent" :budget="r.budget" /></span>
          <span class="num w-10 text-right text-meta text-muted-foreground">{{ percent(r.spent, r.budget) ?? '—' }}<template v-if="percent(r.spent, r.budget) !== null">%</template></span>
        </template>
      </span>
    </div>
    <div :class="cn(grid, 'mt-1 h-12 border-t font-medium')" role="row" data-testid="budget-total">
      <span>{{ t('budget.total') }}</span>
      <span class="num text-right">{{ num(o.total.budget) }}</span>
      <span class="num text-right">{{ num(o.total.spent) }}</span>
      <span :class="cn('num text-right', o.total.remaining < 0 && 'text-destructive')">{{ num(o.total.remaining) }}</span>
      <span />
    </div>
  </div>
</template>
