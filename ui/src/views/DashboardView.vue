<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { PlusIcon } from '@lucide/vue'
import EmptyState from '@/components/EmptyState.vue'
import PageHeader from '@/components/PageHeader.vue'
import AccountDetailCard from '@/components/dashboard/AccountDetailCard.vue'
import AccountsCard from '@/components/dashboard/AccountsCard.vue'
import AllocationSummaryCard from '@/components/dashboard/AllocationSummaryCard.vue'
import BudgetTotalCard from '@/components/dashboard/BudgetTotalCard.vue'
import BudgetWatchCard from '@/components/dashboard/BudgetWatchCard.vue'
import CashflowCard from '@/components/dashboard/CashflowCard.vue'
import DashboardHero from '@/components/dashboard/DashboardHero.vue'
import DueScheduledCard from '@/components/dashboard/DueScheduledCard.vue'
import ExpenseCard from '@/components/dashboard/ExpenseCard.vue'
import NetWorthCard from '@/components/dashboard/NetWorthCard.vue'
import RecentTxnsCard from '@/components/dashboard/RecentTxnsCard.vue'
import { Button } from '@/components/ui/button'
import { Kbd } from '@/components/ui/kbd'
import { defaultAccountId } from '@/lib/accounts'
import { formatRange } from '@/lib/date'
import { withErrorToast } from '@/lib/notify'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'
import { useDashboardStore } from '@/stores/dashboard'
import { useReportsStore } from '@/stores/reports'
import { useScheduledStore } from '@/stores/scheduled'
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
const scheduled = useScheduledStore()

const current = computed(() => reports.cashflow.at(-1))
// Akun terpilih di kartu Akun; jatuh ke akun aset aktif pertama bila belum dipilih atau sudah diarsip.
const picked = ref<string | null>(null)
const selected = computed(() => {
  const active = accounts.overview.accounts.filter((a) => !a.archived)
  const id = picked.value && active.some((a) => a.id === picked.value) ? picked.value : defaultAccountId(active, null)
  return active.find((a) => a.id === id) ?? null
})
const selectedId = computed({ get: () => selected.value?.id ?? null, set: (id) => { picked.value = id } })
const budgetTotal = computed(() => {
  const total = dashboard.budget?.total
  return total && total.available > 0 ? total : null
})
const load = () => withErrorToast(() => Promise.all([dashboard.load(), reports.load(), scheduled.reload()]))

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
    <DashboardHero />
    <EmptyState v-if="dashboard.recent.length === 0" :title="$t('txn.empty')" :description="$t('txn.emptyHint')" class="py-0">
      <Button variant="outline" @click="ui.openNewTxn()"><PlusIcon /> {{ $t('txn.emptyAction') }} <Kbd class="ml-1">N</Kbd></Button>
    </EmptyState>
    <!-- Grid 12 kolom (2 kolom di layar sedang); kartu tanpa data menyembunyikan dirinya sendiri.
         Kartu terakhir di tiap kolom bertumpuk memanjang agar tepi bawah satu baris sejajar. -->
    <div class="grid grid-cols-1 gap-4 md:gap-6 @3xl/main:grid-cols-2 @5xl/main:grid-cols-12">
      <AccountsCard v-model="selectedId" class="@5xl/main:col-span-4" />
      <div class="flex flex-col gap-4 md:gap-6 @5xl/main:col-span-3 [&>:last-child]:flex-1">
        <AccountDetailCard v-if="selected" :account="selected" />
        <BudgetTotalCard v-if="dashboard.budget" :total="dashboard.budget.total" />
      </div>
      <template v-if="dashboard.recent.length">
        <CashflowCard class="@3xl/main:col-span-2 @5xl/main:col-span-5" />
        <RecentTxnsCard :rows="dashboard.recent" class="@3xl/main:col-span-2 @5xl/main:col-span-5" />
      </template>
      <!-- Tanpa pengeluaran tidak ada budget yang perlu dilihat: satu kolom ikut hilang. -->
      <div v-if="dashboard.expenses.length" class="flex flex-col gap-4 md:gap-6 @5xl/main:col-span-4 [&>:last-child]:flex-1">
        <ExpenseCard :rows="dashboard.expenses" :budget="budgetTotal" />
        <BudgetWatchCard v-if="dashboard.budget" :overview="dashboard.budget" />
      </div>
      <NetWorthCard class="@5xl/main:col-span-3" />
    </div>
    <div class="grid grid-cols-1 gap-4 md:gap-6 @3xl/main:grid-cols-2">
      <DueScheduledCard />
      <AllocationSummaryCard v-if="dashboard.allocation" :overview="dashboard.allocation" />
    </div>
  </div>
</template>
