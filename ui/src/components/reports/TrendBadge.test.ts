import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import { i18n } from '@/i18n'
import TrendBadge from './TrendBadge.vue'

const tone = (props: { amount: number; previous: number | undefined; upIsBad?: boolean }) =>
  mount(TrendBadge, { props, global: { plugins: [i18n] } }).find('svg').classes()

describe('TrendBadge', () => {
  it('warna mengikuti makna, termasuk upIsBad yang dikirim undefined', () => {
    expect(tone({ amount: 30, previous: 100, upIsBad: undefined })).toContain('text-expense')
    expect(tone({ amount: 130, previous: 100 })).toContain('text-income')
    expect(tone({ amount: 130, previous: 100, upIsBad: true })).toContain('text-expense')
    expect(tone({ amount: 30, previous: 100, upIsBad: true })).toContain('text-income')
  })
})
