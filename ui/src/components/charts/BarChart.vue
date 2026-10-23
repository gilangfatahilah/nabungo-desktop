<script setup lang="ts">
import { computed, ref } from 'vue'

// Batang berkelompok per label (docs/design/design.md §8.8). Setiap kelompok bisa
// difokus: Enter/Spasi memilih, panah kiri/kanan memindah fokus.
const props = defineProps<{
  labels: string[]
  series: { key: string; label: string; color: string; values: number[] }[]
  label: string
  format: (v: number) => string
  selected?: number
}>()
const emit = defineEmits<{ select: [index: number] }>()

const W = 600
const H = 200
const PAD = { top: 8, right: 8, bottom: 24, left: 48 }
const TICKS = 4
const LABEL_WIDTH = 56
// Porsi lebar kelompok yang diisi batang; sisanya jarak antarkelompok.
const FILL = 0.6

const plotW = W - PAD.left - PAD.right
const range = computed(() => {
  const all = props.series.flatMap((s) => s.values)
  const min = Math.min(0, ...all)
  const max = Math.max(0, ...all)
  return { min, max: max === min ? min + 1 : max }
})
const y = (v: number) => PAD.top + ((range.value.max - v) * (H - PAD.top - PAD.bottom)) / (range.value.max - range.value.min)
const ticks = computed(() =>
  Array.from({ length: TICKS + 1 }, (_, i) => range.value.min + ((range.value.max - range.value.min) * i) / TICKS),
)

const groupW = computed(() => plotW / Math.max(props.labels.length, 1))
const barW = computed(() => (groupW.value * FILL) / Math.max(props.series.length, 1))
const groupX = (i: number) => PAD.left + i * groupW.value
const groups = computed(() =>
  props.labels.map((text, i) => ({
    text,
    i,
    aria: `${text}: ${props.series.map((s) => `${s.label} ${props.format(s.values[i] ?? 0)}`).join(', ')}`,
    bars: props.series.map((s, k) => {
      const v = s.values[i] ?? 0
      return {
        key: s.key,
        color: s.color,
        x: groupX(i) + (groupW.value * (1 - FILL)) / 2 + k * barW.value,
        y: Math.min(y(v), y(0)),
        height: Math.abs(y(v) - y(0)),
      }
    }),
  })),
)
// Label terakhir selalu tampil; sisanya per `step` dihitung mundur darinya.
const step = computed(() => Math.max(1, Math.ceil((props.labels.length * LABEL_WIDTH) / plotW)))
const showLabel = (i: number) => (props.labels.length - 1 - i) % step.value === 0

const els = ref<SVGGElement[]>([])
function onKey(e: KeyboardEvent, i: number) {
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault()
    emit('select', i)
  } else if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
    e.preventDefault()
    els.value[i + (e.key === 'ArrowLeft' ? -1 : 1)]?.focus()
  }
}
</script>

<template>
  <svg :viewBox="`0 0 ${W} ${H}`" role="img" :aria-label="label" class="h-auto w-full text-muted-foreground">
    <rect
      v-if="selected !== undefined && selected >= 0 && selected < labels.length" data-part="selected" :data-index="selected"
      :x="groupX(selected)" :y="PAD.top" :width="groupW" :height="H - PAD.top - PAD.bottom" rx="4" fill="var(--muted)"
    />
    <g v-for="v in ticks" :key="v">
      <line :x1="PAD.left" :x2="W - PAD.right" :y1="y(v)" :y2="y(v)" stroke="var(--border)" stroke-width="1" />
      <text data-part="y-tick" :x="PAD.left - 6" :y="y(v)" text-anchor="end" dominant-baseline="middle" font-size="11" fill="currentColor">{{ format(v) }}</text>
    </g>
    <g
      v-for="g in groups" :key="g.i" ref="els" data-part="group" :data-index="g.i" tabindex="0" role="button"
      :aria-label="g.aria" :aria-pressed="selected === g.i" class="group/bar cursor-pointer outline-none"
      @click="emit('select', g.i)" @keydown="onKey($event, g.i)"
    >
      <rect
        :x="groupX(g.i) + 1" :y="PAD.top + 1" :width="groupW - 2" :height="H - PAD.top - PAD.bottom - 2" rx="4"
        fill="transparent" stroke-width="2" class="stroke-transparent group-focus-visible/bar:stroke-ring"
      />
      <rect
        v-for="b in g.bars" :key="b.key" :data-key="b.key" :data-index="g.i"
        :x="b.x" :y="b.y" :width="barW" :height="b.height" :fill="b.color" rx="1"
      />
      <title>{{ g.aria }}</title>
      <text
        v-if="showLabel(g.i)" data-part="x-label" :x="groupX(g.i) + groupW / 2" :y="H - 6"
        text-anchor="middle" font-size="11" fill="currentColor"
      >{{ g.text }}</text>
    </g>
  </svg>
</template>
