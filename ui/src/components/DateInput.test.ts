import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { i18n } from '@/i18n'
import DateInput from './DateInput.vue'

beforeEach(() => setActivePinia(createPinia()))

const mountInput = (props: { modelValue: string | null; nullable?: boolean }) =>
  mount(DateInput, { props, attrs: { id: 'd' }, global: { plugins: [i18n] }, attachTo: document.body })

describe('DateInput', () => {
  it('menampilkan nilai model dalam format locale, bukan mm/dd/yyyy', () => {
    const w = mountInput({ modelValue: '2026-10-17' })
    expect(w.find<HTMLInputElement>('input#d').element.value).toBe('17 Okt 2026')
    w.unmount()
  })

  it('ketikan di-parse saat blur dan diformat ulang', async () => {
    const w = mountInput({ modelValue: '2026-10-17' })
    const input = w.find<HTMLInputElement>('input#d')
    await input.setValue('5/1/2026')
    await input.trigger('blur')
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual(['2026-01-05'])
    await w.setProps({ modelValue: '2026-01-05' })
    expect(input.element.value).toBe('5 Jan 2026')
    expect(input.attributes('aria-invalid')).toBeUndefined()
    w.unmount()
  })

  it('Enter juga meng-commit', async () => {
    const w = mountInput({ modelValue: '2026-10-17' })
    const input = w.find('input#d')
    await input.setValue('1 Okt')
    await input.trigger('keydown', { key: 'Enter' })
    expect(w.emitted('update:modelValue')?.at(-1)?.[0]).toMatch(/^\d{4}-10-01$/)
    w.unmount()
  })

  it('teks tidak valid ditandai dan model tidak berubah', async () => {
    const w = mountInput({ modelValue: '2026-10-17' })
    const input = w.find('input#d')
    await input.setValue('31/02/2026')
    await input.trigger('blur')
    expect(w.emitted('update:modelValue')).toBeUndefined()
    expect(input.attributes('aria-invalid')).toBe('true')
    w.unmount()
  })

  it('nullable: dikosongkan → null', async () => {
    const w = mountInput({ modelValue: '2026-10-17', nullable: true })
    const input = w.find('input#d')
    await input.setValue('')
    await input.trigger('blur')
    expect(w.emitted('update:modelValue')?.at(-1)).toEqual([null])
    w.unmount()
  })

  it('tidak nullable: dikosongkan dianggap tidak valid', async () => {
    const w = mountInput({ modelValue: '2026-10-17' })
    const input = w.find('input#d')
    await input.setValue('')
    await input.trigger('blur')
    expect(w.emitted('update:modelValue')).toBeUndefined()
    expect(input.attributes('aria-invalid')).toBe('true')
    w.unmount()
  })
})
