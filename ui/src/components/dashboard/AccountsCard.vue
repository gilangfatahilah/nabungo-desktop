<script setup lang="ts">
import { computed } from 'vue'
import { PlusIcon } from '@lucide/vue'
import MoneyText from '@/components/MoneyText.vue'
import { Button } from '@/components/ui/button'
import { Card, CardAction, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { balanceTone, groupAccounts, KIND_ICONS, receivableLabel } from '@/lib/accounts'
import { colorClass } from '@/lib/colors'
import { iconFor } from '@/lib/icons'
import { cn } from '@/lib/utils'
import { useAccountsStore } from '@/stores/accounts'

// Akun aktif sebagai daftar pilihan; akun terpilih mengisi kartu detail di Dasbor (docs/design/design.md §8.1).
// Urutan mengikuti layar Akun (grup lalu sortOrder); saldo dari backend.
const selected = defineModel<string | null>({ required: true })
const accounts = useAccountsStore()
const rows = computed(() => groupAccounts(accounts.overview, { archived: false }).flatMap((s) => s.accounts))
</script>

<template>
  <Card class="gap-4 shadow-xs" data-testid="dashboard-accounts">
    <CardHeader>
      <CardTitle class="text-heading">
        <RouterLink to="/accounts" class="hover:underline">{{ $t('nav.accounts') }}</RouterLink>
      </CardTitle>
      <CardAction>
        <Button as-child variant="ghost" size="icon-sm" :aria-label="$t('dashboard.manageAccounts')">
          <RouterLink to="/accounts"><PlusIcon /></RouterLink>
        </Button>
      </CardAction>
    </CardHeader>
    <CardContent class="max-h-[22rem] overflow-y-auto">
      <div role="radiogroup" :aria-label="$t('nav.accounts')" class="flex flex-col gap-1.5">
        <button
          v-for="a in rows" :key="a.id" type="button" role="radio" data-testid="dashboard-account"
          :aria-checked="a.id === selected"
          :class="cn(
            'flex items-center gap-3 rounded-lg border px-3 py-2 text-left text-sm transition-colors hover:bg-accent focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-none',
            a.id === selected ? 'border-primary bg-primary/5' : 'border-transparent',
          )"
          @click="selected = a.id"
        >
          <span
            :class="cn('size-3.5 shrink-0 rounded-full border', a.id === selected ? 'border-4 border-primary' : 'border-input')"
            aria-hidden="true"
          />
          <span :class="cn('flex size-8 shrink-0 items-center justify-center rounded-md', colorClass(a.color))">
            <component :is="iconFor(a.icon ?? KIND_ICONS[a.kind])" class="size-4" />
          </span>
          <span class="min-w-0 flex-1">
            <span class="block truncate font-medium" :title="a.name">{{ a.name }}</span>
            <span class="block truncate text-meta text-muted-foreground">{{
              a.kind === 'receivable' ? $t(`accounts.${receivableLabel(a.balance)}`) : $t(`accountKinds.${a.kind}`)
            }}</span>
          </span>
          <MoneyText :amount="a.balance" :kind="balanceTone(a)" :currency="false" class="shrink-0 font-medium" />
        </button>
      </div>
    </CardContent>
  </Card>
</template>
