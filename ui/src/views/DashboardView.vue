<script setup lang="ts">
import { onMounted } from 'vue'
import MoneyText from '@/components/MoneyText.vue'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { withErrorToast } from '@/lib/notify'
import { useAccountsStore } from '@/stores/accounts'

const accounts = useAccountsStore()

onMounted(() => {
  if (!accounts.loaded) void withErrorToast(() => accounts.load())
})
</script>

<template>
  <Card class="max-w-sm" data-testid="net-worth-card">
    <CardHeader>
      <CardTitle>{{ $t('dashboard.netWorth') }}</CardTitle>
    </CardHeader>
    <CardContent class="flex flex-col gap-2">
      <MoneyText :amount="accounts.overview.netWorth.net" size="display" />
      <dl class="grid grid-cols-2 gap-1 text-sm">
        <dt class="text-muted-foreground">{{ $t('accounts.assets') }}</dt>
        <dd class="text-right"><MoneyText :amount="accounts.overview.netWorth.assets" /></dd>
        <dt class="text-muted-foreground">{{ $t('accounts.liabilities') }}</dt>
        <dd class="text-right"><MoneyText :amount="accounts.overview.netWorth.liabilities" kind="expense" /></dd>
      </dl>
    </CardContent>
  </Card>
</template>
