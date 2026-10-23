<script setup lang="ts">
import { computed, onMounted, ref, useTemplateRef, watch } from 'vue'

// Tooltip bersama chart SVG (docs/design/design.md §8.8). Posisi dalam pecahan 0–1 dari kotak chart;
// `side` = sisi titik tempat tooltip terbuka; bila tetap melewati tepi kartu terdekat, tooltip digeser masuk.
const props = defineProps<{
  x: number
  y: number
  side: 'left' | 'right'
  title?: string
  rows: { key: string; label: string; color: string; value: string }[]
}>()

const GAP = 8
const el = useTemplateRef<HTMLDivElement>('el')
const shift = ref(0)

// Ukur setelah render lalu geser horizontal agar tetap di dalam kartu (atau jendela) dengan jarak GAP;
// tanpa ini tooltip di kartu sempit tumpah ke kartu tetangga dan tertutup olehnya.
function clamp() {
  if (!el.value) return
  const r = el.value.getBoundingClientRect()
  const b = el.value.closest('[data-slot="card"]')?.getBoundingClientRect() ?? { left: 0, right: window.innerWidth }
  const left = r.left - shift.value
  const right = r.right - shift.value
  shift.value = right > b.right - GAP ? b.right - GAP - right : left < b.left + GAP ? b.left + GAP - left : 0
}
onMounted(clamp)
watch(props, clamp, { flush: 'post' })

const style = computed(() => ({
  left: `${props.x * 100}%`,
  top: `${props.y * 100}%`,
  transform: props.side === 'left' ? 'translate(calc(-100% - 12px), -50%)' : 'translate(12px, -50%)',
  translate: shift.value ? `${shift.value}px` : undefined,
}))
</script>

<template>
  <div
    ref="el" data-part="tooltip" role="status" aria-live="polite" :style="style"
    class="pointer-events-none absolute z-20 flex min-w-32 flex-col gap-1 rounded-md border bg-popover px-3 py-2 text-meta whitespace-nowrap text-popover-foreground shadow-md"
  >
    <p v-if="title" class="font-medium">{{ title }}</p>
    <ul class="flex flex-col gap-0.5">
      <li v-for="r in rows" :key="r.key" class="flex items-center gap-2">
        <span class="size-2 shrink-0 rounded-full" :style="{ background: r.color }" aria-hidden="true" />
        <span class="text-muted-foreground">{{ r.label }}</span>
        <span class="num ml-auto pl-3 font-medium">{{ r.value }}</span>
      </li>
    </ul>
  </div>
</template>
