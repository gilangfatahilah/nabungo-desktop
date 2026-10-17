import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import MoneyText from './MoneyText.vue'

beforeEach(() => setActivePinia(createPinia()))

describe('MoneyText', () => {
  it('menampilkan minus sejati, prefiks mata uang terpisah, dan nada warna', () => {
    const w = mount(MoneyText, { props: { amount: -25_000, kind: 'expense' } })
    expect(w.text()).toBe('−Rp 25.000')
    expect(w.classes()).toContain('text-expense')
    expect(w.classes()).toContain('num')
    expect(w.find('[data-part="currency"]').text()).toBe('Rp')
  })

  it('bisa tanpa prefiks mata uang (kolom angka sidebar)', () => {
    const w = mount(MoneyText, { props: { amount: 4_343_000, currency: false } })
    expect(w.text()).toBe('4.343.000')
    expect(w.find('[data-part="currency"]').exists()).toBe(false)
  })

  it('ukuran display memakai token text-display', () => {
    const w = mount(MoneyText, { props: { amount: 1, size: 'display' } })
    expect(w.classes()).toContain('text-display')
  })
})
