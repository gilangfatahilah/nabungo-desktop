import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import BudgetBar from './BudgetBar.vue'

const bar = (spent: number, budget: number) => mount(BudgetBar, { props: { spent, budget } }).get('[role="progressbar"]')

describe('BudgetBar', () => {
  it('ok sampai 80%, waspada di atasnya, bahaya lewat 100%', () => {
    expect(bar(80, 100).attributes('data-state')).toBe('ok')
    expect(bar(81, 100).attributes('data-state')).toBe('warn')
    expect(bar(100, 100).attributes('data-state')).toBe('warn')
    expect(bar(101, 100).attributes('data-state')).toBe('over')
  })

  it('aria-valuenow = persen dibulatkan; bagian lebih tampil di luar track', () => {
    const b = mount(BudgetBar, { props: { spent: 101, budget: 100 } })
    expect(b.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('101')
    expect(b.find('[data-part="overflow"]').exists()).toBe(true)
    expect(mount(BudgetBar, { props: { spent: 50, budget: 100 } }).find('[data-part="overflow"]').exists()).toBe(false)
  })

  it('budget 0 dengan realisasi tetap bahaya; refund tidak membuat negatif', () => {
    expect(bar(1, 0).attributes('data-state')).toBe('over')
    expect(bar(0, 0).attributes('data-state')).toBe('ok')
    expect(bar(-5, 100).attributes('aria-valuenow')).toBe('0')
  })
})
