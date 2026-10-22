<script setup lang="ts">
import { useElementSize } from '@vueuse/core'
import { computed, useTemplateRef } from 'vue'

// Grafik garis sederhana (docs/design/design.md §8.8); seri `dashed` untuk garis target.
// `area` mengisi 15% alfa warna seri di bawah garis (chart card Dasbor, §7.12).
const props = defineProps<{
  labels: string[]
  series: { key: string; label: string; color: string; values: number[]; dashed?: boolean }[]
  label: string
  format: (v: number) => string
  area?: boolean
}>()

// viewBox mengikuti lebar elemen agar teks sumbu tetap 11px di kartu lebar; 600 bila belum terukur (jsdom).
const { width } = useElementSize(useTemplateRef<SVGSVGElement>('svg'))
const W = computed(() => (width.value > 0 ? Math.round(width.value) : 600))
const H = 200
const PAD = { top: 8, right: 8, bottom: 24, left: 48 }
const TICKS = 4
// Lebar minimum per label sumbu X sebelum label dijarangkan.
const LABEL_WIDTH = 56

const range = computed(() => {
  const all = props.series.flatMap((s) => s.values)
  const min = Math.min(0, ...all)
  const max = Math.max(0, ...all)
  return { min, max: max === min ? min + 1 : max }
})
const x = (i: number) => PAD.left + (props.labels.length > 1 ? (i * (W.value - PAD.left - PAD.right)) / (props.labels.length - 1) : 0)
const y = (v: number) => PAD.top + ((range.value.max - v) * (H - PAD.top - PAD.bottom)) / (range.value.max - range.value.min)

const ticks = computed(() =>
  Array.from({ length: TICKS + 1 }, (_, i) => range.value.min + ((range.value.max - range.value.min) * i) / TICKS),
)
// Label di ujung sumbu ditambatkan ke dalam agar tidak terpotong viewBox.
const anchor = (i: number) => (i === props.labels.length - 1 && i > 0 ? 'end' : 'middle')
const lines = computed(() => props.series.map((s) => ({ ...s, points: s.values.map((v, i) => `${x(i)},${y(v)}`).join(' ') })))
// Poligon area: garis seri lalu kembali sepanjang sumbu nol.
const areas = computed(() => props.area
  ? lines.value.filter((s) => !s.dashed && s.values.length > 1)
    .map((s) => ({ key: s.key, color: s.color, points: `${x(0)},${y(0)} ${s.points} ${x(s.values.length - 1)},${y(0)}` }))
  : [])
// Label terakhir selalu tampil; sisanya per `step` dihitung mundur darinya.
const xLabels = computed(() => {
  const n = props.labels.length
  const step = Math.max(1, Math.ceil((n * LABEL_WIDTH) / (W.value - PAD.left - PAD.right)))
  return props.labels.map((text, i) => ({ text, i })).filter(({ i }) => (n - 1 - i) % step === 0)
})
</script>

<template>
  <svg ref="svg" :viewBox="`0 0 ${W} ${H}`" role="img" :aria-label="label" class="h-auto w-full text-muted-foreground">
    <g v-for="v in ticks" :key="v">
      <line :x1="PAD.left" :x2="W - PAD.right" :y1="y(v)" :y2="y(v)" stroke="var(--border)" stroke-width="1" />
      <text data-part="y-tick" :x="PAD.left - 6" :y="y(v)" text-anchor="end" dominant-baseline="middle" font-size="11" fill="currentColor">{{ format(v) }}</text>
    </g>
    <text
      v-for="l in xLabels" :key="l.i" data-part="x-label" :x="x(l.i)" :y="H - 6"
      :text-anchor="anchor(l.i)" font-size="11" fill="currentColor"
    >{{ l.text }}</text>
    <polygon v-for="a in areas" :key="a.key" data-part="area" :points="a.points" :fill="a.color" fill-opacity="0.15" stroke="none" />
    <polyline
      v-for="s in lines" :key="s.key" :data-key="s.key" :points="s.points" fill="none" :stroke="s.color"
      :stroke-width="s.dashed ? 1.5 : 2" :stroke-dasharray="s.dashed ? '4 4' : undefined" stroke-linejoin="round"
    >
      <title>{{ s.label }}</title>
    </polyline>
  </svg>
</template>
