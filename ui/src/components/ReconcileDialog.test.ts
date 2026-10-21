import { flushPromises, mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import { IpcFailure, type AccountDto } from '@/ipc'
import { formatIdr } from '@/lib/money'
import { useAccountsStore } from '@/stores/accounts'
import ReconcileDialog from './ReconcileDialog.vue'

const toast = vi.hoisted(() => ({ success: vi.fn(), error: vi.fn() }))
vi.mock('vue-sonner', () => ({ toast }))

const base: AccountDto = {
  id: 'a1', name: 'BCA', kind: 'bank', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
  icon: null, color: null, sortOrder: 0, archived: false, balance: 100_000, openingBalance: 0, openingDate: null,
  contact: null, reconciledThrough: null,
}

const $ = (sel: string) => document.body.querySelector(sel)
async function type(sel: string, value: string) {
  const el = $(sel) as HTMLInputElement
  el.value = value
  el.dispatchEvent(new Event('input', { bubbles: true }))
  el.dispatchEvent(new Event('blur'))
  await nextTick()
}
async function submit() {
  ;($('[data-testid="reconcile-form"]') as HTMLFormElement).dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
  await flushPromises()
}
async function open(account: AccountDto) {
  if (!vi.isMockFunction(ipc.accountBalanceAsOf)) vi.spyOn(ipc, 'accountBalanceAsOf').mockResolvedValue(account.balance)
  const w = mount(ReconcileDialog, { props: { open: true, account }, global: { plugins: [i18n] }, attachTo: document.body })
  await flushPromises()
  return w
}

describe('ReconcileDialog', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['Date'], now: new Date('2026-10-17T10:00:00') })
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
    vi.clearAllMocks()
  })
  afterEach(() => {
    document.body.innerHTML = ''
    vi.useRealTimers()
    vi.restoreAllMocks()
  })

  it('bank: saldo riil dikirim apa adanya, selisih → toast penyesuaian, dialog tertutup', async () => {
    const spy = vi.spyOn(useAccountsStore(), 'reconcile').mockResolvedValue({ adjustmentId: 'x', difference: -15_000, marked: 3 })
    const w = await open(base)
    expect(document.body.textContent).toContain(id.reconcile.recorded)
    await type('#rec-balance', '85.000')
    await submit()
    expect(spy).toHaveBeenCalledWith('a1', '2026-10-17', 85_000)
    expect(toast.success).toHaveBeenCalledWith(`Penyesuaian ${formatIdr(-15_000)} dicatat`)
    expect(w.emitted('update:open')?.at(-1)).toEqual([false])
  })

  it('tanggal mundur: saldo tercatat dan isian awal mengikuti saldo per tanggal itu', async () => {
    const asOf = vi.spyOn(ipc, 'accountBalanceAsOf')
      .mockImplementation(async (_id, date) => (date === '2026-10-01' ? 70_000 : 100_000))
    const spy = vi.spyOn(useAccountsStore(), 'reconcile').mockResolvedValue({ adjustmentId: null, difference: 0, marked: 1 })
    await open(base)
    expect($('[data-testid="rec-recorded"]')!.textContent).toContain('100.000')
    await type('#rec-date', '2026-10-01')
    // DateInput mem-parse saat Enter/blur; Enter dipakai karena blur tidak sampai di dalam dialog jsdom.
    $('#rec-date')!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
    await flushPromises()
    await flushPromises()
    expect(asOf).toHaveBeenLastCalledWith('a1', '2026-10-01')
    expect($('[data-testid="rec-recorded"]')!.textContent).toContain('70.000')
    expect(($('#rec-balance') as HTMLInputElement).value).toBe('70000')
    await submit()
    expect(spy).toHaveBeenCalledWith('a1', '2026-10-01', 70_000)
  })

  it('selisih 0 → toast saldo cocok', async () => {
    vi.spyOn(useAccountsStore(), 'reconcile').mockResolvedValue({ adjustmentId: null, difference: 0, marked: 1 })
    await open(base)
    await submit()
    expect(toast.success).toHaveBeenCalledWith(id.reconcile.matched)
  })

  it('kartu kredit: utang diketik positif, dikirim negatif', async () => {
    const spy = vi.spyOn(useAccountsStore(), 'reconcile').mockResolvedValue({ adjustmentId: null, difference: 0, marked: 0 })
    await open({ ...base, kind: 'credit_card', balance: -100_000 })
    expect(($('#rec-balance') as HTMLInputElement).value).toBe('100000')
    await type('#rec-balance', '120.000')
    await submit()
    expect(spy).toHaveBeenCalledWith('a1', '2026-10-17', -120_000)
  })

  it('hutang: arah "Saya berutang" mengirim nilai negatif', async () => {
    const spy = vi.spyOn(useAccountsStore(), 'reconcile').mockResolvedValue({ adjustmentId: null, difference: 0, marked: 0 })
    await open({ ...base, kind: 'receivable', balance: 30_000 })
    ;($('[data-testid="direction-iOwe"]') as HTMLElement).dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0 }))
    await flushPromises()
    await type('#rec-balance', '40.000')
    await submit()
    expect(spy).toHaveBeenCalledWith('a1', '2026-10-17', -40_000)
  })

  it('error backend tampil di dialog', async () => {
    vi.spyOn(useAccountsStore(), 'reconcile').mockRejectedValue(new IpcFailure({ code: 'archived', entity: 'account' }))
    const w = await open(base)
    await submit()
    expect($('[role="alert"]')?.textContent).toBeTruthy()
    expect(w.emitted('update:open')).toBeUndefined()
    expect(toast.success).not.toHaveBeenCalled()
  })
})
