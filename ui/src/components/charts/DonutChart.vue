<script setup lang="ts">
import { computed } from 'vue'

// Donut porsi (docs/design/design.md §8.8); angka lengkap selalu ada di tabel di bawahnya.
const props = defineProps<{ segments: { key: string; label: string; value: number; color: string }[]; label: string }>()

const C = 60
const R = 52
const r = R - 16

function point(radius: number, turn: number) {
  const a = turn * 2 * Math.PI - Math.PI / 2
  return `${(C + radius * Math.cos(a)).toFixed(3)} ${(C + radius * Math.sin(a)).toFixed(3)}`
}

// Busur cincin dari `from` ke `to` (pecahan putaran). Satu segmen penuh dipecah dua agar busurnya tergambar.
function arc(from: number, to: number): string {
  if (to - from >= 1) return `${arc(0, 0.5)} ${arc(0.5, 1)}`
  const large = to - from > 0.5 ? 1 : 0
  return `M ${point(R, from)} A ${R} ${R} 0 ${large} 1 ${point(R, to)} L ${point(r, to)} A ${r} ${r} 0 ${large} 0 ${point(r, from)} Z`
}

const paths = computed(() => {
  const shown = props.segments.filter((s) => s.value > 0)
  const total = shown.reduce((m, s) => m + s.value, 0)
  let at = 0
  return shown.map((s) => {
    const from = at
    at += s.value / total
    return { ...s, d: arc(from, at) }
  })
})
</script>

<template>
  <svg viewBox="0 0 120 120" role="img" :aria-label="label" class="size-full">
    <circle :cx="C" :cy="C" :r="R - 8" fill="none" stroke="var(--muted)" stroke-width="16" />
    <path v-for="s in paths" :key="s.key" :data-key="s.key" :d="s.d" :fill="s.color" stroke="var(--background)" stroke-width="1">
      <title>{{ s.label }}</title>
    </path>
  </svg>
</template>
