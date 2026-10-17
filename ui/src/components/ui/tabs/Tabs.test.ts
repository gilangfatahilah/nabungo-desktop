import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { h } from 'vue'
import { Tabs, TabsList, TabsTrigger } from '.'

// Class shadcn memakai varian `data-horizontal`/`data-vertical`, bukan `data-orientation`.
describe('Tabs', () => {
  const list = () => h(TabsList, () => [h(TabsTrigger, { value: 'a' }, () => 'A')])

  it('horizontal (default) memberi atribut data-horizontal', () => {
    const w = mount(Tabs, { props: { modelValue: 'a' }, slots: { default: list } })
    expect(w.attributes('data-horizontal')).toBe('')
    expect(w.attributes('data-vertical')).toBeUndefined()
  })

  it('vertical memberi atribut data-vertical', () => {
    const w = mount(Tabs, { props: { modelValue: 'a', orientation: 'vertical' }, slots: { default: list } })
    expect(w.attributes('data-vertical')).toBe('')
    expect(w.attributes('data-horizontal')).toBeUndefined()
  })
})
