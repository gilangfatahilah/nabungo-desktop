<script setup lang="ts">
import { computed } from 'vue'

// Porsi aktual sebagai isi bar, target sebagai garis penanda 2px (docs/design/ui-guidelines.md §9.5).
const props = defineProps<{ actualBp: number; targetBp: number; color: string }>()

const clamp = (bp: number) => Math.min(Math.max(bp / 100, 0), 100)
const fill = computed(() => clamp(props.actualBp))
const marker = computed(() => clamp(props.targetBp))
</script>

<template>
  <div class="relative h-1.5 w-full rounded-full bg-muted">
    <div data-part="actual" class="h-full rounded-full" :style="{ width: `${fill}%`, background: color }" />
    <div data-part="target" class="absolute -top-0.75 h-3 w-0.5 -translate-x-1/2 rounded-full bg-foreground" :style="{ left: `${marker}%` }" />
  </div>
</template>
