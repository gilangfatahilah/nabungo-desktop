import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import LineChart from './LineChart.vue'

const labels = ['Agu', 'Sep', 'Okt']
const series = [
  { key: 'a', label: 'Saham', color: 'var(--chart-1)', values: [5000, 6000, 7000] },
  { key: 'a-target', label: 'Target Saham', color: 'var(--chart-1)', values: [6000, 6000, 6000], dashed: true },
]
const format = (v: number) => `${v / 100}%`

describe('LineChart', () => {
  it('menggambar satu polyline per seri; seri target bergaris putus', () => {
    const w = mount(LineChart, { props: { labels, series, label: 'Riwayat', format } })
    expect(w.get('svg').attributes('role')).toBe('img')
    expect(w.get('svg').attributes('aria-label')).toBe('Riwayat')
    const lines = w.findAll('polyline[data-key]')
    expect(lines.map((l) => l.attributes('data-key'))).toEqual(['a', 'a-target'])
    expect(lines[0]!.attributes('stroke-dasharray')).toBeUndefined()
    expect(lines[1]!.attributes('stroke-dasharray')).toBeTruthy()
    expect(lines[0]!.attributes('points')!.split(' ')).toHaveLength(3)
  })

  it('sumbu Y mencakup 0 dan nilai maksimum', () => {
    const w = mount(LineChart, { props: { labels, series, label: 'x', format } })
    const ticks = w.findAll('[data-part="y-tick"]').map((t) => t.text())
    expect(ticks[0]).toBe('0%')
    expect(ticks.at(-1)).toBe('70%')
  })

  it('label X dijarangkan bila periode banyak', () => {
    const many = Array.from({ length: 24 }, (_, i) => `P${i}`)
    const w = mount(LineChart, {
      props: { labels: many, series: [{ key: 'a', label: 'a', color: 'red', values: many.map((_, i) => i) }], label: 'x', format },
    })
    const shown = w.findAll('[data-part="x-label"]')
    expect(shown.length).toBeLessThan(24)
    expect(shown.at(-1)!.text()).toBe('P23')
  })
})
