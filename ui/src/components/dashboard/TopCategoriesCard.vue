<script setup lang="ts">
import { computed } from 'vue'
import MoneyText from '@/components/MoneyText.vue'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import type { CategoryAmountDto } from '@/ipc'
import { colorClass } from '@/lib/colors'
import { iconFor } from '@/lib/icons'
import { cn } from '@/lib/utils'
import { useCategoriesStore } from '@/stores/categories'

// Top kategori pengeluaran periode berjalan; lebar batang relatif ke baris pertama, hanya presentasi (docs/design/design.md §8.1).
const props = defineProps<{ rows: CategoryAmountDto[] }>()
const categories = useCategoriesStore()

const byId = computed(() => new Map(categories.list.map((c) => [c.id, c])))
const max = computed(() => props.rows[0]?.amount ?? 0)
const width = (amount: number) => (max.value > 0 ? `${(amount / max.value) * 100}%` : '0%')
</script>

<template>
  <Card v-if="rows.length" class="gap-4 shadow-xs" data-testid="dashboard-top-categories">
    <CardHeader>
      <CardTitle class="text-heading">
        <RouterLink to="/reports" class="hover:underline">{{ $t('dashboard.topCategories') }}</RouterLink>
      </CardTitle>
    </CardHeader>
    <CardContent class="flex flex-col gap-3">
      <div v-for="r in rows" :key="r.categoryId" class="flex flex-col gap-1.5" data-testid="top-category-row">
        <div class="flex items-center justify-between gap-3 text-sm">
          <span class="flex min-w-0 items-center gap-2">
            <span :class="cn('flex size-6 shrink-0 items-center justify-center rounded-md', colorClass(byId.get(r.categoryId)?.color ?? null))">
              <component :is="iconFor(byId.get(r.categoryId)?.icon ?? null)" class="size-3.5" />
            </span>
            <span class="truncate">{{ byId.get(r.categoryId)?.name }}</span>
          </span>
          <MoneyText :amount="r.amount" :currency="false" class="font-medium" />
        </div>
        <div class="h-1.5 w-full rounded-full bg-muted">
          <div class="h-full rounded-full bg-chart-1" :style="{ width: width(r.amount) }" />
        </div>
      </div>
    </CardContent>
  </Card>
</template>
