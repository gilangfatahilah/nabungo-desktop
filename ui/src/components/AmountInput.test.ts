import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { i18n } from '@/i18n'
import AmountInput from './AmountInput.vue'

const flush = () => new Promise((r) => setTimeout(r, 0))

describe('AmountInput', () => {
  it('blur mem-parse teks dan meng-update model; teks tak valid → error', async () => {
    const w = mount(AmountInput, { props: { modelValue: 0 }, global: { plugins: [i18n] } })
    const input = w.find('input')
    await input.setValue('15000')
    await input.trigger('blur')
    await flush()
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual([15000])
    expect(w.emitted('error')?.at(-1)).toEqual([null])

    await input.setValue('abc')
    await input.trigger('keydown', { key: 'Enter' })
    await flush()
    expect(w.emitted('error')?.at(-1)?.[0]).toBeTypeOf('string')
    expect(input.attributes('aria-invalid')).toBe('true')
  })
})
