import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import TxnFilters from '@/components/txn/TxnFilters.vue'
import { i18n } from '@/i18n'
import { useTransactionsStore } from '@/stores/transactions'

function input(w: ReturnType<typeof mount>, sel: string, value: string) {
  const el = w.find<HTMLInputElement>(sel).element
  el.value = value
  el.dispatchEvent(new Event('input', { bubbles: true }))
}

describe('TxnFilters', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers()
  })
  afterEach(() => vi.useRealTimers())

  it('pencarian di-debounce 250 ms', async () => {
    const store = useTransactionsStore()
    const spy = vi.spyOn(store, 'setQuery').mockResolvedValue()
    const w = mount(TxnFilters, { global: { plugins: [i18n] } })
    input(w, '[data-testid="filter-search"]', 'kop')
    input(w, '[data-testid="filter-search"]', 'kopi')
    await vi.advanceTimersByTimeAsync(200)
    expect(spy).not.toHaveBeenCalled()
    await vi.advanceTimersByTimeAsync(60)
    expect(spy).toHaveBeenCalledTimes(1)
    expect(spy).toHaveBeenCalledWith({ search: 'kopi' })
  })

  it('nominal min memakai parser nominal; kosong = tanpa batas', async () => {
    const store = useTransactionsStore()
    const spy = vi.spyOn(store, 'setQuery').mockResolvedValue()
    const w = mount(TxnFilters, { global: { plugins: [i18n] }, attachTo: document.body })
    // Nominal ada di popover "+ Filter" (§8.6).
    await w.find('[data-testid="filter-more"]').trigger('click')
    await flushPromises()
    const min = document.body.querySelector<HTMLInputElement>('[data-testid="filter-min"]')!
    for (const [text, want] of [['25rb', 25_000], ['', null]] as const) {
      min.value = text
      min.dispatchEvent(new Event('input', { bubbles: true }))
      await flushPromises() // Input (useVModel passive) meneruskan nilai di tick berikutnya.
      min.dispatchEvent(new Event('change', { bubbles: true }))
      await flushPromises()
      expect(spy).toHaveBeenLastCalledWith({ amountMin: want })
    }
    w.unmount()
  })

  it('filter tambahan tampil sebagai chip yang bisa dihapus', async () => {
    const store = useTransactionsStore()
    store.query.tag = 'kantor'
    const spy = vi.spyOn(store, 'setQuery').mockResolvedValue()
    const w = mount(TxnFilters, { global: { plugins: [i18n] } })
    const chip = w.find('[data-testid="chip-tag"]')
    expect(chip.text()).toContain('#kantor')
    await chip.find('button').trigger('click')
    expect(spy).toHaveBeenCalledWith({ tag: null })
  })

  it('jumlah hasil di kanan; hapus semua hanya saat ada filter', async () => {
    const store = useTransactionsStore()
    store.total = 12
    const w = mount(TxnFilters, { global: { plugins: [i18n] } })
    expect(w.find('[data-testid="filter-count"]').text()).toBe('12 transaksi')
    expect(w.find('[data-testid="filter-reset"]').exists()).toBe(false)
  })

  it('reset mengosongkan semua filter', async () => {
    const store = useTransactionsStore()
    store.query.search = 'x'
    store.query.dateFrom = '2026-10-01'
    const w = mount(TxnFilters, { global: { plugins: [i18n] } })
    await w.find('[data-testid="filter-reset"]').trigger('click')
    await flushPromises()
    expect(store.query.search).toBeNull()
    expect(store.query.dateFrom).toBeNull()
    expect((w.find<HTMLInputElement>('[data-testid="filter-search"]').element).value).toBe('')
  })
})
