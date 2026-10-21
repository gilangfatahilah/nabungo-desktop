import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { BudgetLevelDto } from '@/ipc'
import BudgetBar from './BudgetBar.vue'

const mountBar = (spent: number, budget: number, level: BudgetLevelDto) => mount(BudgetBar, { props: { spent, budget, level } })
const bar = (spent: number, budget: number, level: BudgetLevelDto) => mountBar(spent, budget, level).get('[role="progressbar"]')

describe('BudgetBar', () => {
  it('warna mengikuti level dari backend, bukan persen', () => {
    expect(bar(79, 100, 'ok').attributes('data-state')).toBe('ok')
    expect(bar(80, 100, 'warning').attributes('data-state')).toBe('warn')
    expect(bar(100, 100, 'exceeded').attributes('data-state')).toBe('over')
  })

  it('aria-valuenow = persen dibulatkan; bagian lebih tampil di luar track', () => {
    const b = mountBar(101, 100, 'exceeded')
    expect(b.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('101')
    expect(b.find('[data-part="overflow"]').exists()).toBe(true)
    expect(mountBar(50, 100, 'ok').find('[data-part="overflow"]').exists()).toBe(false)
  })

  it('budget 0 dengan realisasi tetap bahaya; refund tidak membuat negatif', () => {
    expect(bar(1, 0, 'exceeded').attributes('aria-valuenow')).toBe('100')
    expect(bar(0, 0, 'ok').attributes('data-state')).toBe('ok')
    expect(bar(-5, 100, 'ok').attributes('aria-valuenow')).toBe('0')
  })

  it('tersedia ≤ 0 berstatus terlampaui tetap tampil penuh walau belum ada realisasi', () => {
    const fill = (b: number) => mountBar(0, b, 'exceeded').get('[role="progressbar"] > div').attributes('style')
    expect(fill(0)).toContain('width: 100%')
    expect(fill(-5)).toContain('width: 100%')
  })
})
