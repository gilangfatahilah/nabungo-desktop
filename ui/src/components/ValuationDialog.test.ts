import { flushPromises, mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import type { AccountDto } from '@/ipc'
import { useAccountsStore } from '@/stores/accounts'
import ValuationDialog from './ValuationDialog.vue'

const $ = (sel: string) => document.body.querySelector(sel)
async function type(sel: string, value: string) {
  const el = $(sel) as HTMLInputElement
  el.value = value
  el.dispatchEvent(new Event('input', { bubbles: true }))
  el.dispatchEvent(new Event('blur'))
  await nextTick()
}

async function investment(): Promise<AccountDto> {
  const store = useAccountsStore()
  const accId = await store.createAccount('investment', {
    name: 'Reksa Dana', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
    icon: null, color: null, openingBalance: 1_000_000, openingDate: '2026-10-01',
  })
  return store.overview.accounts.find((a) => a.id === accId)!
}

async function open(account: AccountDto) {
  const w = mount(ValuationDialog, { props: { open: true, account }, global: { plugins: [i18n] }, attachTo: document.body })
  await flushPromises()
  return w
}

describe('ValuationDialog', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['Date'], now: new Date('2026-10-17T10:00:00') })
    setActivePinia(createPinia())
  })
  afterEach(() => {
    document.body.innerHTML = ''
    vi.useRealTimers()
  })

  it('menyimpan nilai baru memperbarui saldo dan riwayat', async () => {
    const acc = await investment()
    const w = await open(acc)
    expect(($('#val-value') as HTMLInputElement).value).toBe('1000000')
    await type('#val-value', '1.250.000')
    ;($('[data-testid="valuation-form"]') as HTMLFormElement).dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await flushPromises()
    const store = useAccountsStore()
    expect(store.overview.accounts.find((a) => a.id === acc.id)!.balance).toBe(1_250_000)
    const rows = document.body.querySelectorAll('[data-testid="valuation-row"]')
    expect(rows).toHaveLength(1)
    expect(rows[0]!.textContent).toContain('1.250.000')
    expect(rows[0]!.textContent).toContain('+250.000')
    expect(w.emitted('update:open')).toBeUndefined()
    expect(await ipc.valuationHistory(acc.id)).toHaveLength(1)
  })

  it('nilai sama menampilkan error valuation_unchanged', async () => {
    const acc = await investment()
    await open(acc)
    ;($('[data-testid="valuation-form"]') as HTMLFormElement).dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await flushPromises()
    expect($('[role="alert"]')?.textContent).toBe(id.errors.valuation_unchanged)
  })
})
