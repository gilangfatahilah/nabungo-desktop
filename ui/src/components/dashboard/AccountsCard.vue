<script setup lang="ts">
import { computed } from 'vue'
import MoneyText from '@/components/MoneyText.vue'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { balanceTone, groupAccounts } from '@/lib/accounts'
import { useAccountsStore } from '@/stores/accounts'

// Saldo akun aktif per grup (pindahan dari sidebar lama); saldo & subtotal dari backend (docs/design/design.md §8.1).
const accounts = useAccountsStore()
const sections = computed(() => groupAccounts(accounts.overview, { archived: false }).filter((s) => s.accounts.length))
</script>

<template>
  <Card class="gap-4 shadow-xs" data-testid="dashboard-accounts">
    <CardHeader>
      <CardTitle class="text-heading">
        <RouterLink to="/accounts" class="hover:underline">{{ $t('nav.accounts') }}</RouterLink>
      </CardTitle>
    </CardHeader>
    <CardContent class="flex flex-col gap-3">
      <section v-for="s in sections" :key="s.group?.id ?? 'none'">
        <div class="flex h-7 items-center justify-between text-meta text-muted-foreground">
          <span class="truncate">{{ s.group?.name ?? $t('accounts.noGroup') }}</span>
          <MoneyText :amount="s.subtotal" :currency="false" />
        </div>
        <ul>
          <li
            v-for="a in s.accounts" :key="a.id" data-testid="dashboard-account"
            class="flex h-8 items-center justify-between gap-3 text-sm"
          >
            <span class="truncate" :title="a.name">{{ a.name }}</span>
            <MoneyText :amount="a.balance" :kind="balanceTone(a)" :currency="false" class="font-medium" />
          </li>
        </ul>
      </section>
    </CardContent>
  </Card>
</template>
