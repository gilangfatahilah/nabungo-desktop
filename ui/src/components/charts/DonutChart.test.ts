import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import DonutChart from './DonutChart.vue'

const segments = [
  { key: 'a', label: 'Saham', value: 6000, color: 'var(--chart-1)' },
  { key: 'b', label: 'Kosong', value: 0, color: 'var(--chart-2)' },
  { key: 'c', label: 'Liquid', value: 4000, color: 'var(--chart-5)' },
]

describe('DonutChart', () => {
  it('menggambar satu path per segmen bernilai positif', () => {
    const w = mount(DonutChart, { props: { segments, label: 'Porsi aktual' } })
    const svg = w.get('svg')
    expect(svg.attributes('role')).toBe('img')
    expect(svg.attributes('aria-label')).toBe('Porsi aktual')
    expect(svg.attributes('viewBox')).toBe('0 0 120 120')
    expect(w.findAll('path[data-key]').map((p) => p.attributes('data-key'))).toEqual(['a', 'c'])
  })

  it('satu segmen penuh tetap tergambar sebagai cincin', () => {
    const w = mount(DonutChart, { props: { segments: [segments[0]!], label: 'x' } })
    const d = w.get('path[data-key="a"]').attributes('d')
    expect(d).toBeTruthy()
    expect(d).not.toContain('NaN')
  })

  it('tanpa nilai positif tidak menggambar segmen', () => {
    const w = mount(DonutChart, { props: { segments: [segments[1]!], label: 'x' } })
    expect(w.findAll('path[data-key]')).toHaveLength(0)
  })
})
