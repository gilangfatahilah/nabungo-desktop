<script setup lang="ts">
import { computed } from 'vue'
import BudgetBar from '@/components/BudgetBar.vue'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import type { BudgetOverviewDto } from '@/ipc'
import { useCategoriesStore } from '@/stores/categories'

// Maks 5 budget dengan persen tertinggi; level & persen dari backend (docs/design/design.md §8.1).
const WATCH_LIMIT = 5

const props = defineProps<{ overview: BudgetOverviewDto }>()
const categories = useCategoriesStore()

const byId = computed(() => new Map(categories.list.map((c) => [c.id, c])))
const rows = computed(() =>
  props.overview.rows
    .filter((r) => r.available !== null && r.level !== null)
    .sort((a, b) => (b.percent ?? 0) - (a.percent ?? 0))
    .slice(0, WATCH_LIMIT),
)
</script>

<template>
  <Card v-if="rows.length" class="gap-4 shadow-xs" data-testid="budget-watch">
    <CardHeader>
      <CardTitle class="text-heading">
        <RouterLink to="/budget" class="hover:underline">{{ $t('dashboard.budgetWatch') }}</RouterLink>
      </CardTitle>
    </CardHeader>
    <CardContent class="flex flex-col gap-3">
      <div v-for="r in rows" :key="r.categoryId" class="flex flex-col gap-1.5" data-testid="budget-watch-row">
        <div class="flex items-center justify-between gap-3 text-sm">
          <span class="truncate">{{ byId.get(r.categoryId)?.name }}</span>
          <span class="num text-meta text-muted-foreground">{{ r.percent ?? 0 }}%</span>
        </div>
        <BudgetBar :spent="r.spent" :budget="r.available!" :level="r.level!" />
      </div>
    </CardContent>
  </Card>
</template>
