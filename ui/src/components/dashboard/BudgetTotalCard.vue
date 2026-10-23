<script setup lang="ts">
import { computed } from 'vue'
import MoneyText from '@/components/MoneyText.vue'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import type { BudgetTotalDto } from '@/ipc'
import { formatBp } from '@/lib/percent'
import { useSettingsStore } from '@/stores/settings'

// Total budget periode berjalan; angka dari backend, UI hanya menghitung persen & lebar batang (docs/design/design.md §8.1).
// Tanpa budget, kartu tetap tampil sebagai ajakan ke layar Budget.
const props = defineProps<{ total: BudgetTotalDto }>()
const settings = useSettingsStore()

const bp = computed(() => (props.total.available > 0 ? Math.round((props.total.spent * 10000) / props.total.available) : 0))
const empty = computed(() => props.total.available <= 0)
const over = computed(() => props.total.spent > props.total.available)
</script>

<template>
  <Card class="gap-3 shadow-xs" data-testid="dashboard-budget-total">
    <CardHeader>
      <CardTitle class="text-heading">
        <RouterLink to="/budget" class="hover:underline">{{ $t('dashboard.budgetTotal') }}</RouterLink>
      </CardTitle>
      <CardDescription v-if="empty">{{ $t('dashboard.budgetEmpty') }}</CardDescription>
    </CardHeader>
    <CardContent v-if="empty" data-testid="budget-total-empty">
      <Button as-child variant="outline" size="sm"><RouterLink to="/budget">{{ $t('dashboard.budgetSetUp') }}</RouterLink></Button>
    </CardContent>
    <CardContent v-else class="flex flex-col gap-2">
      <div class="flex items-baseline justify-between gap-3 text-sm">
        <span class="flex flex-wrap items-baseline gap-1 text-muted-foreground">
          <MoneyText :amount="total.spent" :currency="false" class="font-medium text-foreground" />
          {{ $t('dashboard.budgetOf') }}
          <MoneyText :amount="total.available" :currency="false" />
        </span>
        <span class="num font-medium" :class="over ? 'text-expense' : ''">{{ formatBp(bp, settings.locale) }}</span>
      </div>
      <div class="h-2 w-full overflow-hidden rounded-full bg-primary/15" aria-hidden="true">
        <div class="h-full rounded-full" :class="over ? 'bg-expense' : 'bg-primary'" :style="{ width: `${Math.min(bp, 10000) / 100}%` }" />
      </div>
    </CardContent>
  </Card>
</template>
