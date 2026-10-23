<script setup lang="ts">
import { computed } from 'vue'
import { ArrowDownLeft, ArrowLeftRight, ArrowUpRight } from '@lucide/vue'
import MoneyText from '@/components/MoneyText.vue'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import type { AccountDto, TxnFormKindDto } from '@/ipc'
import { balanceTone, receivableLabel } from '@/lib/accounts'
import { formatDateInput } from '@/lib/date'
import { useAccountsStore } from '@/stores/accounts'
import { useSettingsStore } from '@/stores/settings'
import { useUiStore } from '@/stores/ui'

// Detail akun terpilih + aksi cepat yang membuka sheet transaksi dengan jenis & akun terisi (docs/design/design.md §8.1).
const props = defineProps<{ account: AccountDto }>()
const accounts = useAccountsStore()
const settings = useSettingsStore()
const ui = useUiStore()

const ACTIONS: { kind: TxnFormKindDto; icon: typeof ArrowDownLeft }[] = [
  { kind: 'income', icon: ArrowDownLeft },
  { kind: 'transfer', icon: ArrowLeftRight },
  { kind: 'expense', icon: ArrowUpRight },
]

const group = computed(() => accounts.overview.groups.find((g) => g.id === props.account.groupId)?.name ?? null)
const kindLabel = computed(() => props.account.kind === 'receivable'
  ? `accounts.${receivableLabel(props.account.balance)}`
  : `accountKinds.${props.account.kind}`)
</script>

<template>
  <Card class="gap-4 shadow-xs" data-testid="account-detail">
    <CardContent class="flex flex-col gap-4">
      <div class="grid grid-cols-3 gap-2" role="group" :aria-label="$t('dashboard.quickActions')">
        <Button
          v-for="a in ACTIONS" :key="a.kind" variant="outline" size="sm" class="h-auto flex-col gap-1 py-2 text-meta"
          :data-testid="`quick-${a.kind}`" @click="ui.openNewTxn({ kind: a.kind, accountId: account.id })"
        >
          <component :is="a.icon" class="size-4" aria-hidden="true" />{{ $t(`txn.kinds.${a.kind}`) }}
        </Button>
      </div>
      <div class="flex flex-col gap-1">
        <span class="truncate text-meta text-muted-foreground">{{ account.name }} · {{ $t(kindLabel) }}</span>
        <MoneyText :amount="account.balance" :kind="balanceTone(account)" class="text-title font-semibold" />
      </div>
      <div class="flex flex-wrap items-center gap-1.5 text-meta text-muted-foreground">
        <Badge v-if="group" variant="secondary">{{ group }}</Badge>
        <Badge variant="outline">{{ account.reconciledThrough
          ? $t('reconcile.through', { date: formatDateInput(account.reconciledThrough, settings.locale) })
          : $t('dashboard.notReconciled') }}</Badge>
        <span v-if="account.creditLimit" class="flex items-center gap-1">
          {{ $t('accounts.creditLimit') }} <MoneyText :amount="account.creditLimit" :currency="false" />
        </span>
        <span v-if="account.statementDay">· {{ $t('accounts.statementDay') }} {{ account.statementDay }}</span>
      </div>
    </CardContent>
  </Card>
</template>
