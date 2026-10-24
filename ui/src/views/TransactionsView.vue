<script setup lang="ts">
import { onMounted } from 'vue'
import TxnFilters from '@/components/txn/TxnFilters.vue'
import TxnList from '@/components/txn/TxnList.vue'
import TxnTable from '@/components/txn/TxnTable.vue'
import { withErrorToast } from '@/lib/notify'
import { useTransactionsStore } from '@/stores/transactions'

const store = useTransactionsStore()

onMounted(() => withErrorToast(() => store.reload()))
</script>

<template>
  <!-- Riwayat atau Tabel (preferensi per perangkat). Mengisi sisa tinggi layout agar list virtual punya area scroll sendiri; header & toolbar tetap terlihat.
       Tanpa kotak ber-border: daftar langsung di Lembar (design.md §8.2). -->
  <div class="flex min-h-0 flex-1 flex-col">
    <TxnFilters class="border-b" />
    <TxnTable v-if="store.view === 'table'" />
    <TxnList v-else />
  </div>
</template>
