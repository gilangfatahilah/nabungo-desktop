<script setup lang="ts">
import PageHeader from '@/components/PageHeader.vue'
import { onMounted } from 'vue'
import TxnFilters from '@/components/txn/TxnFilters.vue'
import TxnList from '@/components/txn/TxnList.vue'
import { withErrorToast } from '@/lib/notify'
import { useTransactionsStore } from '@/stores/transactions'

const store = useTransactionsStore()

onMounted(() => withErrorToast(() => store.reload()))
</script>

<template>
  <PageHeader :title="$t('nav.transactions')" />
  <!-- Mengisi sisa tinggi layout agar list virtual punya area scroll sendiri; header & filter tetap terlihat. -->
  <div class="flex min-h-0 flex-1 flex-col overflow-hidden rounded-xl border">
    <TxnFilters />
    <TxnList />
  </div>
</template>
