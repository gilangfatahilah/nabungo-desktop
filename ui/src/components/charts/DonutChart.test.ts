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

  it('tooltip: hover segmen menampilkan nama, nominal, dan persen; segmen lain meredup', async () => {
    const w = mount(DonutChart, { props: { segments, label: 'x', format: (v: number) => `Rp${v}` } })
    await w.get('path[data-key="c"]').trigger('mouseenter')
    expect(w.emitted('update:active')).toEqual([['c']])
    await w.setProps({ active: 'c' })
    const tip = w.get('[data-part="tooltip"]')
    expect(tip.text()).toContain('Liquid')
    expect(tip.text()).toContain('Rp4000')
    expect(tip.text()).toContain('40%')
    expect(w.get('path[data-key="a"]').attributes('opacity')).toBe('0.4')
    expect(w.get('path[data-key="c"]').attributes('opacity')).toBeUndefined()
  })

  it('segmen aktif dari luar (legenda) hanya menyorot, tanpa tooltip', () => {
    const w = mount(DonutChart, { props: { segments, label: 'x', active: 'a' } })
    expect(w.get('path[data-key="c"]').attributes('opacity')).toBe('0.4')
    expect(w.find('[data-part="tooltip"]').exists()).toBe(false)
  })

  it('tooltip: keyboard — fokus ke segmen pertama, panah memutar, Escape menutup', async () => {
    const w = mount(DonutChart, { props: { segments, label: 'x', 'onUpdate:active': (v: string | null) => w.setProps({ active: v }) } })
    await w.trigger('focus')
    expect(w.get('[data-part="tooltip"]').text()).toContain('Saham')
    expect(w.get('[data-part="tooltip"]').text()).toContain('60%')
    await w.trigger('keydown', { key: 'ArrowRight' })
    expect(w.get('[data-part="tooltip"]').text()).toContain('Liquid')
    await w.trigger('keydown', { key: 'ArrowRight' })
    expect(w.get('[data-part="tooltip"]').text()).toContain('Saham')
    await w.trigger('keydown', { key: 'Escape' })
    expect(w.find('[data-part="tooltip"]').exists()).toBe(false)
  })
})
