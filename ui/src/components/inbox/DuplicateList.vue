<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import MoneyText from '@/components/MoneyText.vue'
import { Button } from '@/components/ui/button'
import type { DuplicateTxnDto } from '@/ipc'
import { formatDateInput } from '@/lib/date'
import { useAccountsStore } from '@/stores/accounts'
import { useSettingsStore } from '@/stores/settings'

// Kandidat transaksi yang sudah tercatat untuk satu draft email; `Hubungkan` tidak membuat transaksi baru.
defineProps<{ duplicates: DuplicateTxnDto[]; busy: boolean }>()
defineEmits<{ link: [txnId: string] }>()

const { t } = useI18n()
const accounts = useAccountsStore()
const settings = useSettingsStore()
const accountName = (id: string) => accounts.overview.accounts.find((a) => a.id === id)?.name ?? ''
</script>

<template>
  <ul class="flex flex-col">
    <li v-for="d in duplicates" :key="d.id" data-testid="duplicate-row" class="flex min-h-12 items-center gap-3 border-b py-1.5 last:border-b-0">
      <div class="min-w-0 flex-1">
        <p class="truncate font-medium">{{ d.payee || '—' }}</p>
        <p class="flex gap-3 text-meta text-muted-foreground">
          <span class="num">{{ formatDateInput(d.date, settings.locale) }}</span>
          <span class="truncate">{{ accountName(d.accountId) }}</span>
        </p>
      </div>
      <MoneyText :amount="d.amount" class="shrink-0 text-right" />
      <Button size="sm" variant="outline" :disabled="busy" @click="$emit('link', d.id)">{{ t('inbox.link') }}</Button>
    </li>
  </ul>
</template>
