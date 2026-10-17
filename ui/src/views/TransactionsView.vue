<script setup lang="ts">
import { onMounted } from 'vue'
import TxnFilters from '@/components/txn/TxnFilters.vue'
import TxnList from '@/components/txn/TxnList.vue'
import { withErrorToast } from '@/lib/notify'
import { useTransactionsStore } from '@/stores/transactions'

const store = useTransactionsStore()

onMounted(() => withErrorToast(() => store.reload()))
</script>

<template>
  <!-- Tinggi dibatasi viewport (dikurangi padding layout p-6) agar list virtual punya area scroll sendiri. -->
  <div class="flex h-[calc(100svh-3rem)] min-h-0 flex-col overflow-hidden rounded-lg border">
    <h1 class="sr-only">{{ $t('nav.transactions') }}</h1>
    <TxnFilters />
    <TxnList />
  </div>
</template>
