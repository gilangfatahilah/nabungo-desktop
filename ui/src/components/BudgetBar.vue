<script setup lang="ts">
import { computed } from 'vue'
import { cn } from '@/lib/utils'

// Bar realisasi budget (docs/design/ui-guidelines.md §9.5): pudar ≤80%, waspada >80%, bahaya >100%.
const props = defineProps<{ spent: number; budget: number }>()

const percent = computed(() => {
  const spent = Math.max(props.spent, 0)
  if (props.budget <= 0) return spent > 0 ? Infinity : 0
  return (spent * 100) / props.budget
})
const state = computed(() => (percent.value > 100 ? 'over' : percent.value > 80 ? 'warn' : 'ok'))
const now = computed(() => (Number.isFinite(percent.value) ? Math.round(percent.value) : 100))
const fill = computed(() => Math.min(percent.value, 100))
// Lebihnya digambar menjulur ke luar track, dibatasi agar tidak menabrak kolom lain.
const overflow = computed(() => Math.min(percent.value - 100, 25))
const color = computed(() => ({ ok: 'bg-muted-foreground/60', warn: 'bg-warning', over: 'bg-destructive' })[state.value])
</script>

<template>
  <div
    role="progressbar" :aria-valuenow="now" aria-valuemin="0" aria-valuemax="100" :data-state="state"
    class="relative h-1.5 w-full rounded-full bg-muted"
  >
    <div :class="cn('h-full rounded-full', color)" :style="{ width: `${fill}%` }" />
    <div
      v-if="state === 'over'" data-part="overflow"
      :class="cn('absolute top-0 left-full h-full rounded-r-full opacity-60', color)" :style="{ width: `${overflow}%` }"
    />
  </div>
</template>
