<script setup lang="ts">
import { computed } from 'vue'
import AllocationBar from '@/components/allocation/AllocationBar.vue'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import type { AllocationOverviewDto } from '@/ipc'
import { formatBp } from '@/lib/percent'
import { useSettingsStore } from '@/stores/settings'

// Bucket root saja; porsi & target dari backend. Palet sama dengan layar Alokasi (docs/design/design.md §8.1, §8.8).
const PALETTE = ['var(--chart-1)', 'var(--chart-2)', 'var(--chart-3)', 'var(--chart-4)']

const props = defineProps<{ overview: AllocationOverviewDto }>()
const settings = useSettingsStore()

const pct = (bp: number) => formatBp(bp, settings.locale)
const actuals = computed(() => new Map(props.overview.actuals.map((a) => [a.bucketId, a])))
const roots = computed(() => props.overview.buckets.filter((b) => !b.parentId))
const warn = computed(() => props.overview.warnings.length > 0 || props.overview.amountWarnings.length > 0)
</script>

<template>
  <Card v-if="roots.length" class="gap-4 shadow-xs" data-testid="dashboard-allocation">
    <CardHeader>
      <CardTitle class="text-heading">
        <RouterLink to="/allocation" class="hover:underline">{{ $t('dashboard.allocation') }}</RouterLink>
      </CardTitle>
    </CardHeader>
    <CardContent class="flex flex-col gap-3">
      <div v-for="(b, i) in roots" :key="b.id" class="flex flex-col gap-1.5" data-testid="allocation-summary-row">
        <div class="flex items-center justify-between gap-3 text-sm">
          <span class="truncate">{{ b.name }}</span>
          <span class="num shrink-0 text-meta text-muted-foreground">
            {{ $t('dashboard.allocationRow', {
              actual: pct(actuals.get(b.id)?.actualBp ?? 0), target: pct(actuals.get(b.id)?.targetBp ?? 0),
            }) }}
          </span>
        </div>
        <AllocationBar
          :actual-bp="actuals.get(b.id)?.actualBp ?? 0" :target-bp="actuals.get(b.id)?.targetBp ?? 0"
          :color="PALETTE[i % PALETTE.length]!"
        />
      </div>
      <p v-if="overview.unallocatedBp > 0" class="text-meta text-muted-foreground">
        {{ $t('dashboard.unallocated', { percent: pct(overview.unallocatedBp) }) }}
      </p>
      <p v-if="warn" class="text-meta text-warning" role="status">{{ $t('dashboard.allocationWarning') }}</p>
    </CardContent>
  </Card>
</template>
