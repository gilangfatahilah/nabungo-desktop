<script setup lang="ts">
import { ChevronLeft, ChevronRight, CircleAlert, Copy, EllipsisVertical, TriangleAlert } from '@lucide/vue'
import { computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from 'vue-sonner'
import BudgetBar from '@/components/BudgetBar.vue'
import EmptyState from '@/components/EmptyState.vue'
import PageHeader from '@/components/PageHeader.vue'
import TableScroll from '@/components/TableScroll.vue'
import BudgetAmountCell from '@/components/budget/BudgetAmountCell.vue'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuCheckboxItem, DropdownMenuContent, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { useMoneyFormat } from '@/composables/useMoneyFormat'
import { formatPeriod, formatRange } from '@/lib/date'
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
// Kolom: kategori | budget | bawaan | terpakai | sisa | [per hari] | bar + persen | menu.
// Per hari hanya untuk periode berjalan (`daysLeft !== null`).
const grid = computed(() => o.value?.daysLeft == null
  ? 'grid grid-cols-[minmax(0,1fr)_8rem_7rem_8rem_8rem_10rem_2rem] items-center gap-4 px-2'
  : 'grid grid-cols-[minmax(0,1fr)_8rem_7rem_8rem_8rem_7rem_10rem_2rem] items-center gap-4 px-2')
const showPerDay = computed(() => o.value?.daysLeft != null)
const summary = computed(() => ({
  warning: o.value?.rows.filter((r) => r.level === 'warning').length ?? 0,
  exceeded: o.value?.rows.filter((r) => r.level === 'exceeded').length ?? 0,
}))

onMounted(() => {
  void withErrorToast(async () => {
    await categories.ensureLoaded()
    await store.init()
  })
})

const txns = useTransactionsStore()
watch(() => txns.ledgerVersion, () => {
  if (store.period) void withErrorToast(() => store.load(store.period!))
})

const { num } = useMoneyFormat()
// Bawaan bertanda; `—` bila tidak ada yang dibawa.
const carry = (v: number) => (v === 0 ? '—' : v > 0 ? `+${num(v)}` : num(v))

function save(categoryId: string, amount: number | null) {
  void withErrorToast(() => store.setAmount(categoryId, amount))
}

function setRollover(categoryId: string, on: boolean) {
  void withErrorToast(() => store.setRollover(categoryId, on))
}

function copy() {
  void withErrorToast(async () => {
    const n = await store.copyPrevious()
    toast(n > 0 ? t('budget.copied', { n }) : t('budget.copiedNone'))
  })
}
</script>

<template>
  <PageHeader>
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

  <p
    v-if="o && (summary.warning || summary.exceeded)" data-testid="budget-summary"
    class="mb-3 flex flex-wrap items-center gap-x-5 gap-y-1 text-sm"
  >
    <span v-if="summary.exceeded" class="flex items-center gap-1.5 text-destructive">
      <CircleAlert class="size-4" /> {{ t('budget.summaryExceeded', { n: summary.exceeded }) }}
    </span>
    <span v-if="summary.warning" class="flex items-center gap-1.5 text-warning">
      <TriangleAlert class="size-4" /> {{ t('budget.summaryWarning', { n: summary.warning }) }}
    </span>
  </p>

  <EmptyState v-if="o && !o.rows.length" :title="t('budget.empty')" :description="t('budget.emptyHint')" />
  <TableScroll v-else-if="o" :label="t('nav.budget')">
    <div class="flex flex-col" role="table">
    <div :class="cn(grid, 'h-8 text-meta text-muted-foreground')" role="row">
      <span role="columnheader">{{ t('budget.category') }}</span>
      <span role="columnheader" class="text-right">{{ t('budget.amount') }}</span>
      <span role="columnheader" class="text-right">{{ t('budget.carry') }}</span>
      <span role="columnheader" class="text-right">{{ t('budget.spent') }}</span>
      <span role="columnheader" class="text-right">{{ t('budget.remaining') }}</span>
      <span v-if="showPerDay" role="columnheader" class="text-right">{{ t('budget.perDay') }}</span>
      <span role="columnheader" />
      <span role="columnheader" />
    </div>
    <div
      v-for="r in o.rows" :key="r.categoryId" role="row" :data-testid="`budget-row-${r.categoryId}`"
      :class="cn(grid, 'group/row rounded-lg hover:bg-muted/50', r.parentId ? 'h-10' : 'h-12')"
    >
      <span role="cell" :class="cn('truncate', r.parentId ? 'pl-6' : 'font-medium')">{{ names.get(r.categoryId) ?? '' }}</span>
      <span role="cell">
        <BudgetAmountCell
          :category-id="r.categoryId" :name="names.get(r.categoryId) ?? ''" :amount="r.budget"
          @save="save(r.categoryId, $event)" @error="toast.error($event)"
        />
      </span>
      <span
        :data-testid="`budget-carry-${r.categoryId}`"
        :class="cn('num text-right', r.carry === 0 ? 'text-muted-foreground' : r.carry < 0 && 'text-destructive')"
      >{{ carry(r.carry) }}</span>
      <span role="cell" class="num text-right">{{ num(r.spent) }}</span>
      <span
        :data-testid="`budget-remaining-${r.categoryId}`"
        :class="cn('num text-right', r.remaining === null && 'text-muted-foreground', r.remaining !== null && r.remaining < 0 && 'text-destructive')"
      >{{ r.remaining === null ? '—' : num(r.remaining) }}</span>
      <span role="cell" v-if="showPerDay" :class="cn('num text-right', r.perDay === null && 'text-muted-foreground')">
        {{ r.perDay === null ? '—' : num(r.perDay) }}
      </span>
      <span role="cell" class="flex items-center gap-3">
        <template v-if="r.available !== null && r.level !== null">
          <!-- Ruang kanan untuk bagian lebih yang menjulur keluar track (maks 25% lebar track). -->
          <span class="flex-1 pr-[20%]"><BudgetBar :spent="r.spent" :budget="r.available" :level="r.level" :label="names.get(r.categoryId) ?? ''" /></span>
          <span class="num w-10 text-right text-meta text-muted-foreground">{{ r.percent ?? '—' }}<template v-if="r.percent !== null">%</template></span>
        </template>
      </span>
      <span role="cell">
        <DropdownMenu v-if="r.budget !== null">
          <DropdownMenuTrigger as-child>
            <Button
              variant="ghost" size="icon" :aria-label="t('budget.rowMenu', { name: names.get(r.categoryId) ?? '' })"
              class="size-8 opacity-0 group-hover/row:opacity-100 group-focus-within/row:opacity-100 focus-visible:opacity-100 data-[state=open]:opacity-100"
            >
              <EllipsisVertical />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuCheckboxItem :model-value="r.rollover" @update:model-value="setRollover(r.categoryId, $event === true)">
              {{ t('budget.rollover') }}
            </DropdownMenuCheckboxItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </span>
    </div>
    <div :class="cn(grid, 'mt-1 h-12 border-t font-medium')" role="row" data-testid="budget-total">
      <span role="cell">{{ t('budget.total') }}</span>
      <span role="cell" class="num text-right">{{ num(o.total.budget) }}</span>
      <span role="cell" :class="cn('num text-right', o.total.carry === 0 ? 'text-muted-foreground' : o.total.carry < 0 && 'text-destructive')">{{ carry(o.total.carry) }}</span>
      <span role="cell" class="num text-right">{{ num(o.total.spent) }}</span>
      <span role="cell" :class="cn('num text-right', o.total.remaining < 0 && 'text-destructive')">{{ num(o.total.remaining) }}</span>
      <span role="cell" v-if="showPerDay" class="num text-right">{{ o.total.perDay === null ? '—' : num(o.total.perDay) }}</span>
      <span role="cell" />
      <span role="cell" />
    </div>
    </div>
  </TableScroll>
</template>
