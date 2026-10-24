import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { nextTick } from 'vue'
import TableScroll from './TableScroll.vue'

function setWidths(el: Element, scroll: number, client: number) {
  Object.defineProperty(el, 'scrollWidth', { configurable: true, value: scroll })
  Object.defineProperty(el, 'clientWidth', { configurable: true, value: client })
}

describe('TableScroll', () => {
  it('region berlabel; tabindex hanya saat isinya lebih lebar dari wadah', async () => {
    const w = mount(TableScroll, { props: { label: 'Tabel budget' }, slots: { default: '<div role="table" />' }, attachTo: document.body })
    const region = w.get('[role="region"]')
    expect(region.attributes('aria-label')).toBe('Tabel budget')
    setWidths(region.element, 400, 400)
    window.dispatchEvent(new Event('resize'))
    await nextTick()
    expect(region.attributes('tabindex')).toBeUndefined()
    setWidths(region.element, 900, 400)
    window.dispatchEvent(new Event('resize'))
    await nextTick()
    expect(region.attributes('tabindex')).toBe('0')
    w.unmount()
  })
})
