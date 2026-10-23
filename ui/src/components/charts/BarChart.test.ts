import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import BarChart from './BarChart.vue'

const labels = ['Agu', 'Sep', 'Okt']
const series = [
  { key: 'in', label: 'Masuk', color: 'var(--chart-2)', values: [100, 200, 300] },
  { key: 'out', label: 'Keluar', color: 'var(--chart-3)', values: [50, 100, 0] },
]
const format = (v: number) => String(v)
const props = { labels, series, label: 'Cashflow', format }

describe('BarChart', () => {
  it('satu rect per seri per label', () => {
    const w = mount(BarChart, { props })
    expect(w.get('svg').attributes('aria-label')).toBe('Cashflow')
    const rects = w.findAll('rect[data-key]')
    expect(rects).toHaveLength(6)
    expect(rects.map((r) => `${r.attributes('data-key')}:${r.attributes('data-index')}`)).toEqual([
      'in:0', 'out:0', 'in:1', 'out:1', 'in:2', 'out:2',
    ])
  })

  it('tinggi batang proporsional terhadap nilai', () => {
    const w = mount(BarChart, { props })
    const h = (key: string, i: number) => Number(w.get(`rect[data-key="${key}"][data-index="${i}"]`).attributes('height'))
    expect(h('in', 1)).toBeCloseTo(2 * h('in', 0))
    expect(h('in', 2)).toBeCloseTo(3 * h('in', 0))
    expect(h('out', 2)).toBe(0)
  })

  it('Enter memilih kelompok; panah memindah fokus; kelompok terpilih berlatar', async () => {
    const w = mount(BarChart, { props: { ...props, selected: 2 }, attachTo: document.body })
    const groups = w.findAll('[data-part="group"]')
    expect(groups).toHaveLength(3)
    expect(groups.every((g) => g.attributes('tabindex') === '0')).toBe(true)
    expect(w.findAll('[data-part="selected"]')).toHaveLength(1)
    expect(w.get('[data-part="selected"]').attributes('data-index')).toBe('2')
    await groups[1]!.trigger('keydown', { key: 'Enter' })
    await groups[0]!.trigger('keydown', { key: ' ' })
    await groups[2]!.trigger('click')
    expect(w.emitted('select')).toEqual([[1], [0], [2]])
    ;(groups[1]!.element as SVGGElement).focus()
    await groups[1]!.trigger('keydown', { key: 'ArrowRight' })
    expect(document.activeElement).toBe(groups[2]!.element)
    await groups[2]!.trigger('keydown', { key: 'ArrowLeft' })
    expect(document.activeElement).toBe(groups[1]!.element)
    w.unmount()
  })

  it('tooltip: hover kelompok menampilkan label dan nilai; kelompok lain meredup', async () => {
    const w = mount(BarChart, { props: { ...props, valueFormat: (v: number) => `Rp${v}` } })
    const groups = w.findAll('[data-part="group"]')
    await groups[1]!.trigger('mouseenter')
    const tip = w.get('[data-part="tooltip"]')
    expect(tip.text()).toContain('Sep')
    expect(tip.text()).toContain('Masuk')
    expect(tip.text()).toContain('Rp200')
    expect(tip.text()).toContain('Rp100')
    expect(w.get('rect[data-key="in"][data-index="0"]').attributes('opacity')).toBe('0.4')
    expect(w.get('rect[data-key="in"][data-index="1"]').attributes('opacity')).toBeUndefined()
    await w.trigger('mouseleave')
    expect(w.find('[data-part="tooltip"]').exists()).toBe(false)
  })

  it('tooltip: fokus kelompok lewat keyboard ikut menampilkan tooltip', async () => {
    const w = mount(BarChart, { props })
    await w.findAll('[data-part="group"]')[0]!.trigger('focus')
    expect(w.get('[data-part="tooltip"]').text()).toContain('Agu')
    await w.findAll('[data-part="group"]')[0]!.trigger('blur')
    expect(w.find('[data-part="tooltip"]').exists()).toBe(false)
  })
})
