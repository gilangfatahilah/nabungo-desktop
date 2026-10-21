import { beforeEach, describe, expect, it, vi } from 'vitest'
import { showBudgetAlerts } from './budgetAlerts'

const toast = vi.hoisted(() => ({ warning: vi.fn(), error: vi.fn() }))
vi.mock('vue-sonner', () => ({ toast }))

const names = new Map([['c1', 'Makan & Minum']])

describe('showBudgetAlerts', () => {
  beforeEach(() => vi.clearAllMocks())

  it('warning → toast.warning dengan persen', () => {
    showBudgetAlerts([{ categoryId: 'c1', period: '2026-10', level: 'warning', percent: 85 }], names)
    expect(toast.warning).toHaveBeenCalledExactlyOnceWith('Budget Makan & Minum sudah 85% terpakai')
    expect(toast.error).not.toHaveBeenCalled()
  })

  it('exceeded → toast.error, dengan atau tanpa persen', () => {
    showBudgetAlerts([
      { categoryId: 'c1', period: '2026-10', level: 'exceeded', percent: 105 },
      { categoryId: 'c1', period: '2026-11', level: 'exceeded', percent: null },
    ], names)
    expect(toast.error).toHaveBeenNthCalledWith(1, 'Budget Makan & Minum terlampaui (105%)')
    expect(toast.error).toHaveBeenNthCalledWith(2, 'Budget Makan & Minum terlampaui')
  })

  it('daftar kosong → tidak ada toast', () => {
    showBudgetAlerts([], names)
    expect(toast.warning).not.toHaveBeenCalled()
    expect(toast.error).not.toHaveBeenCalled()
  })
})
