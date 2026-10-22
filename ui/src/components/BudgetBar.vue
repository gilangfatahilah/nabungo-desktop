<script setup lang="ts">
import { computed } from 'vue'
import type { BudgetLevelDto } from '@/ipc'
import { cn } from '@/lib/utils'

// Bar realisasi budget (docs/design/design.md §7.10): warna dari `level` backend
// (pudar < 80%, waspada ≥ 80%, bahaya ≥ 100%); `budget` = tersedia (budget + bawaan).
const props = defineProps<{ spent: number; budget: number; level: BudgetLevelDto }>()

const percent = computed(() => {
  const spent = Math.max(props.spent, 0)
  if (props.budget <= 0) return spent > 0 ? Infinity : 0
  return (spent * 100) / props.budget
})
const state = computed(() => ({ ok: 'ok', warning: 'warn', exceeded: 'over' } as const)[props.level])
const now = computed(() => (Number.isFinite(percent.value) ? Math.round(percent.value) : 100))
// Tersedia ≤ 0 yang sudah terlampaui (bawaan negatif) digambar penuh agar warna bahaya terlihat.
const fill = computed(() => (props.budget <= 0 && props.level === 'exceeded' ? 100 : Math.min(percent.value, 100)))
// Lebihnya digambar menjulur ke luar track, dibatasi agar tidak menabrak kolom lain.
const overflow = computed(() => Math.min(Math.max(percent.value - 100, 0), 25))
const color = computed(() => ({ ok: 'bg-muted-foreground/60', warn: 'bg-warning', over: 'bg-destructive' })[state.value])
</script>

<template>
  <div
    role="progressbar" :aria-valuenow="now" aria-valuemin="0" aria-valuemax="100" :data-state="state"
    class="relative h-1.5 w-full rounded-full bg-muted"
  >
    <div :class="cn('h-full rounded-full', color)" :style="{ width: `${fill}%` }" />
    <div
      v-if="percent > 100" data-part="overflow"
      :class="cn('absolute top-0 left-full h-full rounded-r-full opacity-60', color)" :style="{ width: `${overflow}%` }"
    />
  </div>
</template>
