<script setup lang="ts">
import { onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ChevronLeft, ChevronRight } from '@lucide/vue'
import PageHeader from '@/components/PageHeader.vue'
import BiggestChangesCard from '@/components/reports/BiggestChangesCard.vue'
import CashflowReportCard from '@/components/reports/CashflowReportCard.vue'
import CategoryReportCard from '@/components/reports/CategoryReportCard.vue'
import CategoryTrendCard from '@/components/reports/CategoryTrendCard.vue'
import NetWorthReportCard from '@/components/reports/NetWorthReportCard.vue'
import ReportsKpis from '@/components/reports/ReportsKpis.vue'
import { Button } from '@/components/ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { formatPeriod, formatRange } from '@/lib/date'
import { withErrorToast } from '@/lib/notify'
import { REPORT_RANGES, type ReportRange } from '@/lib/reportStats'
import { useCategoriesStore } from '@/stores/categories'
import { useReportsStore } from '@/stores/reports'
import { useSettingsStore } from '@/stores/settings'
import { useTransactionsStore } from '@/stores/transactions'

// Layar Laporan (docs/design/design.md §8.8): PageHeader = rentang periode fokus + pemilih `‹ periode ›`
// dan rentang chart; di bawahnya grid kartu. Kartu tanpa data menyembunyikan dirinya sendiri.
const store = useReportsStore()
const categories = useCategoriesStore()
const settings = useSettingsStore()
const txns = useTransactionsStore()
const { t } = useI18n()

const shift = (step: -1 | 1) => withErrorToast(() => store.shift(step))
function setRange(v: unknown) {
  const r = REPORT_RANGES.find((x) => String(x) === v)
  if (r) void withErrorToast(() => store.setRange(r as ReportRange))
}

onMounted(() => {
  void withErrorToast(() => Promise.all([store.loadReport(), categories.load()]))
})
watch(() => txns.ledgerVersion, () => void withErrorToast(() => store.loadReport()))
</script>

<template>
  <PageHeader>
    <template v-if="store.focus">{{ t('dashboard.period', { range: formatRange(store.focus.from, store.focus.to, settings.locale) }) }}</template>
    <template #actions>
      <div class="flex h-7 items-center rounded-[min(var(--radius-md),10px)] border border-input dark:bg-input-fill/30" role="group" :aria-label="t('reports.focusPeriod')">
        <Button variant="ghost" size="icon-xs" :disabled="!store.canPrev" :aria-label="t('reports.prevPeriod')" data-testid="period-prev" @click="shift(-1)">
          <ChevronLeft aria-hidden="true" />
        </Button>
        <span class="num min-w-24 px-1 text-center text-sm" aria-live="polite" data-testid="period-label">
          {{ store.period ? formatPeriod(store.period, settings.locale) : '' }}
        </span>
        <Button variant="ghost" size="icon-xs" :disabled="!store.canNext" :aria-label="t('reports.nextPeriod')" data-testid="period-next" @click="shift(1)">
          <ChevronRight aria-hidden="true" />
        </Button>
      </div>
      <Select :model-value="String(store.range)" @update:model-value="setRange">
        <SelectTrigger size="sm" class="w-40" data-testid="reports-range" :aria-label="t('reports.range')"><SelectValue /></SelectTrigger>
        <SelectContent align="end">
          <SelectItem v-for="r in REPORT_RANGES" :key="r" :value="String(r)">{{ t(`reports.ranges.${r}`) }}</SelectItem>
        </SelectContent>
      </Select>
    </template>
  </PageHeader>

  <div class="flex flex-col gap-4 md:gap-6">
    <ReportsKpis />
    <div class="grid grid-cols-1 gap-4 md:gap-6 @3xl/main:grid-cols-12">
      <CashflowReportCard class="@3xl/main:col-span-12 @5xl/main:col-span-8" />
      <BiggestChangesCard class="@3xl/main:col-span-12 @5xl/main:col-span-4" />
      <CategoryReportCard class="@3xl/main:col-span-7" />
      <CategoryTrendCard class="@3xl/main:col-span-5" />
      <NetWorthReportCard class="@3xl/main:col-span-12" />
    </div>
  </div>
</template>
