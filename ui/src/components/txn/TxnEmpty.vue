<script setup lang="ts">
import { PlusIcon } from '@lucide/vue'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import EmptyState from '@/components/EmptyState.vue'
import { Button } from '@/components/ui/button'
import { Kbd } from '@/components/ui/kbd'
import { activeFilterCount } from '@/lib/txnFilter'
import { withErrorToast } from '@/lib/notify'
import { useTransactionsStore } from '@/stores/transactions'
import { useUiStore } from '@/stores/ui'

// Empty state layar Transaksi (design.md §7.7): kosong karena filter vs belum ada transaksi.
const { t } = useI18n()
const store = useTransactionsStore()
const ui = useUiStore()
const hasFilter = computed(() => activeFilterCount(store.query) > 0)
</script>

<template>
  <EmptyState v-if="hasFilter" :title="t('txn.emptyFiltered')" class="py-4">
    <Button variant="outline" data-testid="txn-empty-clear" @click="withErrorToast(() => store.clear())">{{ t('txn.filters.clearAll') }}</Button>
  </EmptyState>
  <EmptyState v-else :title="t('txn.empty')" :description="t('txn.emptyHint')" class="py-4">
    <Button data-testid="txn-empty-new" @click="ui.openNewTxn()"><PlusIcon /> {{ t('txn.emptyAction') }} <Kbd class="ml-1">N</Kbd></Button>
  </EmptyState>
</template>
