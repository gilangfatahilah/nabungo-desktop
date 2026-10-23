<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { PlusIcon } from '@lucide/vue'
import EmptyState from '@/components/EmptyState.vue'
import PageHeader from '@/components/PageHeader.vue'
import AccountsCard from '@/components/dashboard/AccountsCard.vue'
import BudgetWatchCard from '@/components/dashboard/BudgetWatchCard.vue'
import CashflowCard from '@/components/dashboard/CashflowCard.vue'
import RecentTxnsCard from '@/components/dashboard/RecentTxnsCard.vue'
import SectionCards from '@/components/dashboard/SectionCards.vue'
import { Button } from '@/components/ui/button'
import { Kbd } from '@/components/ui/kbd'
import { formatRange } from '@/lib/date'
import { withErrorToast } from '@/lib/notify'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'
import { useDashboardStore } from '@/stores/dashboard'
import { useReportsStore } from '@/stores/reports'
import { useSettingsStore } from '@/stores/settings'
import { useTransactionsStore } from '@/stores/transactions'
import { useUiStore } from '@/stores/ui'

// Pola layar Dasbor (docs/design/design.md §8.1).
const dashboard = useDashboardStore()
const reports = useReportsStore()
const accounts = useAccountsStore()
const categories = useCategoriesStore()
const settings = useSettingsStore()
const ui = useUiStore()

const current = computed(() => reports.cashflow.at(-1))
const load = () => withErrorToast(() => Promise.all([dashboard.load(), reports.load()]))

onMounted(() => {
  void load()
  void withErrorToast(() => Promise.all([accounts.loaded ? null : accounts.load(), categories.loaded ? null : categories.load()]))
})

const txns = useTransactionsStore()
watch(() => txns.ledgerVersion, () => void load())
</script>

<template>
  <PageHeader>
    <template v-if="current">{{ $t('dashboard.period', { range: formatRange(current.from, current.to, settings.locale) }) }}</template>
    <template #actions>
      <Button data-testid="dashboard-new-txn" @click="ui.openNewTxn()"><PlusIcon /> {{ $t('sidebar.newTransaction') }}</Button>
    </template>
  </PageHeader>
  <div class="flex flex-col gap-4 md:gap-6">
    <SectionCards />
    <EmptyState v-if="dashboard.recent.length === 0" :title="$t('txn.empty')" :description="$t('txn.emptyHint')" class="py-0">
      <Button variant="outline" @click="ui.openNewTxn()"><PlusIcon /> {{ $t('txn.emptyAction') }} <Kbd class="ml-1">N</Kbd></Button>
    </EmptyState>
    <template v-else>
      <CashflowCard />
      <div class="grid grid-cols-1 gap-4 md:gap-6 @5xl/main:grid-cols-3">
        <RecentTxnsCard :rows="dashboard.recent" class="@5xl/main:col-span-2" />
        <div class="flex flex-col gap-4 md:gap-6">
          <AccountsCard />
          <BudgetWatchCard v-if="dashboard.budget" :overview="dashboard.budget" />
        </div>
      </div>
    </template>
  </div>
</template>
