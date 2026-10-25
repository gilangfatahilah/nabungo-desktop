<script setup lang="ts">
import { computed, ref } from 'vue'
import type { LocaleDto } from '@/ipc'
import { formatBp } from '@/lib/percent'
import ChartTooltip from './ChartTooltip.vue'

// Donut porsi (docs/design/design.md §8.8); angka lengkap selalu ada di tabel/legenda di sampingnya.
// Segmen aktif (hover, keyboard, atau legenda lewat `v-model:active`) menonjol; lainnya meredup. Tooltip hanya
// untuk hover/fokus di donut sendiri; dari legenda cukup sorotan karena angkanya sudah ada di sana.
const props = withDefaults(defineProps<{
  segments: { key: string; label: string; value: number; color: string }[]
  label: string
  /** Format nominal di tooltip; tanpa ini tooltip hanya menampilkan persen. */
  format?: (v: number) => string
  locale?: LocaleDto
}>(), { format: undefined, locale: 'id' })
const active = defineModel<string | null>('active', { default: null })
const own = ref<string | null>(null)
function set(key: string | null) {
  own.value = key
  active.value = key
}

const C = 60
const R = 52
const r = R - 16
// Jari-jari luar segmen aktif.
const R_ACTIVE = R + 4

function point(radius: number, turn: number) {
  const a = turn * 2 * Math.PI - Math.PI / 2
  return `${(C + radius * Math.cos(a)).toFixed(3)} ${(C + radius * Math.sin(a)).toFixed(3)}`
}

// Busur cincin dari `from` ke `to` (pecahan putaran). Satu segmen penuh dipecah dua agar busurnya tergambar.
function arc(from: number, to: number, outer = R): string {
  if (to - from >= 1) return `${arc(0, 0.5, outer)} ${arc(0.5, 1, outer)}`
  const large = to - from > 0.5 ? 1 : 0
  return `M ${point(outer, from)} A ${outer} ${outer} 0 ${large} 1 ${point(outer, to)} L ${point(r, to)} A ${r} ${r} 0 ${large} 0 ${point(r, from)} Z`
}

const paths = computed(() => {
  const shown = props.segments.filter((s) => s.value > 0)
  const total = shown.reduce((m, s) => m + s.value, 0)
  let at = 0
  return shown.map((s) => {
    const from = at
    at += s.value / total
    return { ...s, from, to: at, bp: Math.round((s.value * 10000) / total), d: arc(from, at), dActive: arc(from, at, R_ACTIVE) }
  })
})

const tooltip = computed(() => {
  const s = paths.value.find((p) => p.key === own.value)
  if (!s) return null
  const share = formatBp(s.bp, props.locale)
  // Ditambatkan di tepi kanan donut setinggi titik tengah busur, terbuka ke kanan (arah legenda); bila kartunya
  // sempit, ChartTooltip menggesernya masuk ke kartu.
  const a = ((s.from + s.to) / 2) * 2 * Math.PI - Math.PI / 2
  return {
    y: (C + R_ACTIVE * Math.sin(a)) / 120,
    // Dua baris agar ringkas: nama sebagai judul, lalu nominal + persen (tanpa `format`: nama + persen).
    title: props.format ? s.label : undefined,
    rows: [{ key: s.key, label: props.format ? props.format(s.value) : s.label, color: s.color, value: share }],
  }
})

function onKey(e: KeyboardEvent) {
  const n = paths.value.length
  if (!n) return
  const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[e.key]
  if (step !== undefined) {
    e.preventDefault()
    const i = paths.value.findIndex((p) => p.key === active.value)
    set(paths.value[i < 0 ? 0 : (i + step + n) % n]!.key)
  } else if (e.key === 'Escape') set(null)
}
</script>

<template>
  <div
    class="relative size-full rounded-full outline-none focus-visible:ring-2 focus-visible:ring-ring" tabindex="0"
    role="group" :aria-label="label"
    @mouseleave="set(null)" @focus="set(active ?? paths[0]?.key ?? null)" @blur="set(null)" @keydown="onKey"
  >
    <svg viewBox="0 0 120 120" role="img" :aria-label="label" class="size-full overflow-visible">
      <circle :cx="C" :cy="C" :r="R - 8" fill="none" stroke="var(--muted)" stroke-width="16" />
      <path
        v-for="s in paths" :key="s.key" :data-key="s.key" :d="active === s.key ? s.dActive : s.d" :fill="s.color"
        stroke="var(--background)" stroke-width="1" class="transition-opacity"
        :opacity="active !== null && active !== s.key ? 0.4 : undefined"
        @mouseenter="set(s.key)"
      />
    </svg>
    <ChartTooltip v-if="tooltip" :x="1" :y="tooltip.y" side="right" :title="tooltip.title" :rows="tooltip.rows" />
  </div>
</template>
