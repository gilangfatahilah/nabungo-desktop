import { mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick } from 'vue'
import ChartTooltip from './ChartTooltip.vue'

const rows = [{ key: 'a', label: 'Bank', color: 'red', value: '100%' }]

// jsdom tidak mengukur layout: kartu 0–300px, tooltip seolah dirender di `tip` (sebelum digeser).
function mockRects(tip: { left: number; right: number }) {
  return vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
    const r = this.dataset.slot === 'card' ? { left: 0, right: 300 } : this.dataset.part === 'tooltip' ? tip : { left: 0, right: 0 }
    return { ...r, top: 0, bottom: 0, width: r.right - r.left, height: 0, x: r.left, y: 0, toJSON: () => ({}) } as DOMRect
  })
}

function mountInCard(props: InstanceType<typeof ChartTooltip>['$props']) {
  const Host = defineComponent(() => () => h('div', { 'data-slot': 'card' }, [h(ChartTooltip, props)]))
  return mount(Host, { attachTo: document.body })
}

describe('ChartTooltip', () => {
  afterEach(() => vi.restoreAllMocks())

  it('digeser ke kiri bila melewati tepi kanan kartu (jarak 8px)', async () => {
    mockRects({ left: 250, right: 400 })
    const w = mountInCard({ x: 1, y: 0.5, side: 'right', rows })
    await nextTick()
    expect(w.get('[data-part="tooltip"]').attributes('style')).toContain('translate: -108px')
    w.unmount()
  })

  it('digeser ke kanan bila melewati tepi kiri kartu', async () => {
    mockRects({ left: -40, right: 110 })
    const w = mountInCard({ x: 0, y: 0.5, side: 'left', rows })
    await nextTick()
    expect(w.get('[data-part="tooltip"]').attributes('style')).toContain('translate: 48px')
    w.unmount()
  })

  it('tidak digeser bila muat di dalam kartu', async () => {
    mockRects({ left: 50, right: 200 })
    const w = mountInCard({ x: 0.5, y: 0.5, side: 'right', rows })
    await nextTick()
    expect(w.get('[data-part="tooltip"]').attributes('style')).not.toContain('translate:')
    w.unmount()
  })
})
