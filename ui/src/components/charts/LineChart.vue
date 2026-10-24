<script setup lang="ts">
import { useElementSize } from '@vueuse/core'
import { computed, ref, useTemplateRef } from 'vue'
import { useRootFontScale } from '@/composables/useRootFontScale'
import ChartTooltip from './ChartTooltip.vue'
import { yTicks } from './axis'
import { monotonePath } from './curve'

// Grafik garis sederhana (docs/design/design.md §8.8); seri `dashed` untuk garis target.
// `area` mengisi 15% alfa warna seri di bawah garis (chart card Dasbor, §7.12); `smooth` melengkungkan
// garis (kubik monoton, tidak melampaui nilai data) dan dirender sebagai <path>, bukan <polyline>.
// Hover/fokus menampilkan garis kursor + tooltip periode terdekat; panah/Home/End memindah, Escape menutup.
const props = defineProps<{
  labels: string[]
  series: { key: string; label: string; color: string; values: number[]; dashed?: boolean }[]
  label: string
  format: (v: number) => string
  area?: boolean
  smooth?: boolean
  /** Format nilai di tooltip (default: `format`, mis. nominal penuh vs label sumbu ringkas). */
  valueFormat?: (v: number) => string
}>()

// viewBox mengikuti lebar elemen agar teks sumbu tetap 11px di kartu lebar; 600 bila belum terukur (jsdom).
// Dibagi skala ukuran teks: teks, padding, dan tinggi chart ikut membesar seperti elemen rem.
const svg = useTemplateRef<SVGSVGElement>('svg')
const { width } = useElementSize(svg)
const scale = useRootFontScale()
const W = computed(() => (width.value > 0 ? Math.round(width.value / scale.value) : 600))
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
  yTicks(range.value.min, range.value.max, TICKS, props.series.every((s) => s.values.every((v) => v === 0))))
// Label di ujung sumbu ditambatkan ke dalam agar tidak terpotong viewBox.
const anchor = (i: number) => (i === props.labels.length - 1 && i > 0 ? 'end' : 'middle')
const lines = computed(() => props.series.map((s) => ({
  ...s,
  points: s.values.map((v, i) => `${x(i)},${y(v)}`).join(' '),
  path: props.smooth ? monotonePath(s.values.map((v, i) => ({ x: x(i), y: y(v) }))) : '',
})))
// Area: garis seri lalu kembali sepanjang sumbu nol.
const areas = computed(() => props.area
  ? lines.value.filter((s) => !s.dashed && s.values.length > 1).map((s) => {
    const end = x(s.values.length - 1)
    return {
      key: s.key,
      color: s.color,
      d: props.smooth ? `${s.path} L${end},${y(0)} L${x(0)},${y(0)} Z` : `M${x(0)},${y(0)} L${s.points.replaceAll(' ', ' L')} L${end},${y(0)} Z`,
    }
  })
  : [])
// Label terakhir selalu tampil; sisanya per `step` dihitung mundur darinya.
const xLabels = computed(() => {
  const n = props.labels.length
  const step = Math.max(1, Math.ceil((n * LABEL_WIDTH) / (W.value - PAD.left - PAD.right)))
  return props.labels.map((text, i) => ({ text, i })).filter(({ i }) => (n - 1 - i) % step === 0)
})

const active = ref<number | null>(null)
const last = computed(() => props.labels.length - 1)
function onMove(e: MouseEvent) {
  const rect = svg.value?.getBoundingClientRect()
  if (!rect?.width || last.value < 0) return
  const px = ((e.clientX - rect.left) * W.value) / rect.width
  const i = last.value > 0 ? Math.round(((px - PAD.left) * last.value) / (W.value - PAD.left - PAD.right)) : 0
  active.value = Math.min(Math.max(i, 0), last.value)
}
function onKey(e: KeyboardEvent) {
  const cur = active.value ?? last.value
  const next = { ArrowLeft: cur - 1, ArrowRight: cur + 1, Home: 0, End: last.value }[e.key]
  if (next !== undefined) {
    e.preventDefault()
    active.value = Math.min(Math.max(next, 0), last.value)
  } else if (e.key === 'Escape') active.value = null
}
const tooltip = computed(() => {
  const i = active.value
  if (i === null || i > last.value) return null
  const fmt = props.valueFormat ?? props.format
  return {
    x: x(i) / W.value,
    title: props.labels[i],
    rows: props.series.map((s) => ({ key: s.key, label: s.label, color: s.color, value: fmt(s.values[i] ?? 0) })),
  }
})
</script>

<template>
  <div
    class="relative rounded-md outline-none focus-visible:ring-2 focus-visible:ring-ring" tabindex="0" role="group" :aria-label="label"
    @mousemove="onMove" @mouseleave="active = null" @focus="active ??= last" @blur="active = null" @keydown="onKey"
  >
    <svg ref="svg" :viewBox="`0 0 ${W} ${H}`" role="img" :aria-label="label" class="h-auto w-full text-muted-foreground">
      <g v-for="v in ticks" :key="v">
        <line :x1="PAD.left" :x2="W - PAD.right" :y1="y(v)" :y2="y(v)" stroke="var(--border)" stroke-width="1" />
        <text data-part="y-tick" :x="PAD.left - 6" :y="y(v)" text-anchor="end" dominant-baseline="middle" font-size="11" fill="currentColor">{{ format(v) }}</text>
      </g>
      <text
        v-for="l in xLabels" :key="l.i" data-part="x-label" :x="x(l.i)" :y="H - 6"
        :text-anchor="anchor(l.i)" font-size="11" fill="currentColor"
      >{{ l.text }}</text>
      <path v-for="a in areas" :key="a.key" data-part="area" :d="a.d" :fill="a.color" fill-opacity="0.15" stroke="none" />
      <template v-for="s in lines" :key="s.key">
        <path
          v-if="smooth" :data-key="s.key" :d="s.path" fill="none" :stroke="s.color"
          :stroke-width="s.dashed ? 1.5 : 2" :stroke-dasharray="s.dashed ? '4 4' : undefined" stroke-linecap="round"
        />
        <polyline
          v-else :data-key="s.key" :points="s.points" fill="none" :stroke="s.color"
          :stroke-width="s.dashed ? 1.5 : 2" :stroke-dasharray="s.dashed ? '4 4' : undefined" stroke-linejoin="round"
        />
      </template>
      <template v-if="active !== null && tooltip">
        <line data-part="cursor" :x1="x(active)" :x2="x(active)" :y1="PAD.top" :y2="H - PAD.bottom" stroke="var(--muted-foreground)" stroke-width="1" stroke-dasharray="3 3" />
        <circle
          v-for="s in series" :key="s.key" data-part="dot" :cx="x(active)" :cy="y(s.values[active] ?? 0)" r="3.5"
          :fill="s.color" stroke="var(--background)" stroke-width="1.5"
        />
      </template>
    </svg>
    <ChartTooltip v-if="tooltip" :x="tooltip.x" :y="0.4" :side="tooltip.x > 0.5 ? 'left' : 'right'" :title="tooltip.title" :rows="tooltip.rows" />
  </div>
</template>
