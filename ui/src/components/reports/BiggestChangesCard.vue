<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { TrendingDown, TrendingUp } from '@lucide/vue'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { useMoneyFormat } from '@/composables/useMoneyFormat'
import { colorClass } from '@/lib/colors'
import { iconFor } from '@/lib/icons'
import { withErrorToast } from '@/lib/notify'
import { biggestChanges, type CategoryChange } from '@/lib/reportStats'
import { cn } from '@/lib/utils'
import { useCategoriesStore } from '@/stores/categories'
import { useReportsStore } from '@/stores/reports'
import TrendBadge from './TrendBadge.vue'

// Kategori root dengan kenaikan/penurunan terbesar vs periode lalu (docs/design/design.md §8.8).
// Klik = pilih untuk Tren kategori. Tanpa perubahan kartu tidak tampil.
const LIMIT = 3

const store = useReportsStore()
const categories = useCategoriesStore()
const { t } = useI18n()

const byId = computed(() => new Map(categories.list.map((c) => [c.id, c])))
const changes = computed(() => biggestChanges(store.comparison, LIMIT))
const groups = computed(() => [
  { key: 'up', label: t('reports.increased'), icon: TrendingUp, rows: changes.value.up },
  { key: 'down', label: t('reports.decreased'), icon: TrendingDown, rows: changes.value.down },
].filter((g) => g.rows.length))
const upIsBad = computed(() => store.kind === 'expense')

const { num } = useMoneyFormat()
const delta = (c: CategoryChange) => (c.delta > 0 ? '+' : '−') + num(Math.abs(c.delta))
const pick = (id: string) => withErrorToast(() => store.setTrendCategory(id))
</script>

<template>
  <Card v-if="groups.length" class="@container/card gap-4 shadow-xs" data-testid="reports-changes">
    <CardHeader>
      <CardTitle class="text-heading">{{ t('reports.biggestChanges') }}</CardTitle>
      <CardDescription>{{ t(`reports.biggestChangesHint.${store.kind}`) }}</CardDescription>
    </CardHeader>
    <CardContent class="flex flex-col gap-4">
      <section v-for="g in groups" :key="g.key" class="flex flex-col gap-1" :data-part="g.key">
        <h3 class="flex items-center gap-1.5 text-meta text-muted-foreground">
          <component :is="g.icon" class="size-3.5" aria-hidden="true" />{{ g.label }}
        </h3>
        <ul class="flex flex-col">
          <li v-for="c in g.rows" :key="c.categoryId">
            <button
              type="button" data-testid="change-row"
              class="flex min-h-10 w-full items-center gap-3 rounded-lg px-1 text-left outline-none hover:bg-muted/50 focus-visible:ring-2 focus-visible:ring-ring"
              @click="pick(c.categoryId)"
            >
              <span :class="cn('flex size-7 shrink-0 items-center justify-center rounded-md', colorClass(byId.get(c.categoryId)?.color ?? null))">
                <component :is="iconFor(byId.get(c.categoryId)?.icon ?? null)" class="size-4" aria-hidden="true" />
              </span>
              <span class="min-w-0 flex-1 truncate">{{ byId.get(c.categoryId)?.name ?? '' }}</span>
              <span
                class="num shrink-0 text-sm font-medium" data-part="delta"
                :class="(c.delta > 0) === upIsBad ? 'text-expense' : 'text-income'"
              >{{ delta(c) }}</span>
              <TrendBadge :amount="c.amount" :previous="c.previous" :up-is-bad="upIsBad" />
            </button>
          </li>
        </ul>
      </section>
    </CardContent>
  </Card>
</template>
