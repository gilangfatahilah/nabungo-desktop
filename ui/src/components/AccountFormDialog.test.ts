import { flushPromises, mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import type { AccountDto } from '@/ipc'
import { useAccountsStore } from '@/stores/accounts'
import AccountFormDialog from './AccountFormDialog.vue'

function account(kind: AccountDto['kind']): AccountDto {
  return {
    id: 'a1', name: 'Kartu', kind, groupId: null, includeInNetWorth: true, creditLimit: 5_000_000, statementDay: 20,
    icon: null, color: null, sortOrder: 1, archived: false, balance: -100_000, openingBalance: 0, openingDate: null, contact: null, reconciledThrough: null,
  }
}

// Dialog di-teleport ke body: query lewat document.
const $ = (sel: string) => document.body.querySelector(sel)
const key = (el: Element, k: string) => el.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true }))
// Input shadcn memakai useVModel passive: model ikut ter-update di flush berikutnya.
async function type(sel: string, value: string) {
  const el = $(sel) as HTMLInputElement
  el.value = value
  el.dispatchEvent(new Event('input', { bubbles: true }))
  await nextTick()
}

async function open(props: { account?: AccountDto } = {}) {
  const w = mount(AccountFormDialog, {
    props: { open: true, ...props },
    global: { plugins: [i18n] },
    attachTo: document.body,
  })
  await flushPromises()
  return w
}

describe('AccountFormDialog', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
  })
  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('field kartu kredit hanya untuk jenis credit_card', async () => {
    const create = await open()
    expect($('[data-testid="credit-limit"]')).toBeNull()
    create.unmount()
    const bank = await open({ account: account('bank') })
    expect($('[data-testid="credit-limit"]')).toBeNull()
    bank.unmount()
    await open({ account: account('credit_card') })
    expect($('[data-testid="credit-limit"]')).not.toBeNull()
    expect($('[data-testid="statement-day"]')).not.toBeNull()
  })

  it('mode edit menonaktifkan pilihan jenis', async () => {
    await open({ account: account('bank') })
    expect($('[data-testid="kind"]')?.hasAttribute('disabled')).toBe(true)
  })

  it('nama kosong → pesan name_empty, createAccount tidak dipanggil', async () => {
    const store = useAccountsStore()
    const spy = vi.spyOn(store, 'createAccount')
    await open()
    ;($('form') as HTMLFormElement).dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await flushPromises()
    expect(document.body.textContent).toContain(id.errors.name_empty)
    expect(spy).not.toHaveBeenCalled()
  })

  it('nominal yang baru diketik ikut tersimpan walau submit tanpa blur', async () => {
    const store = useAccountsStore()
    const spy = vi.spyOn(store, 'createAccount').mockResolvedValue('x')
    await open()
    await type('#acc-name', 'BCA')
    await type('#acc-opening', '2000000')
    ;($('form') as HTMLFormElement).dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await flushPromises()
    expect(spy).toHaveBeenCalledTimes(1)
    expect(spy.mock.calls[0]![1].openingBalance).toBe(2_000_000)
  })

  it('akun liabilitas: utang diketik positif, disimpan negatif', async () => {
    const store = useAccountsStore()
    const spy = vi.spyOn(store, 'createAccount').mockResolvedValue('x')
    await open()
    key($('[data-testid="kind"]')!, 'Enter')
    await flushPromises()
    const opt = [...document.body.querySelectorAll('[role="option"]')].find(
      (o) => o.textContent?.trim() === id.accountKinds.credit_card,
    )!
    key(opt, 'Enter')
    await flushPromises()
    expect(document.body.textContent).toContain(id.accounts.openingDebt)
    await type('#acc-name', 'Kartu')
    await type('#acc-opening', '2000000')
    ;($('form') as HTMLFormElement).dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await flushPromises()
    expect(spy.mock.calls[0]![0]).toBe('credit_card')
    expect(spy.mock.calls[0]![1].openingBalance).toBe(-2_000_000)
  })

  it('edit akun liabilitas menampilkan utang sebagai angka positif', async () => {
    await open({ account: { ...account('loan'), openingBalance: -100_000 } })
    expect(($('#acc-opening') as HTMLInputElement).value).toBe('100000')
  })
})


describe('AccountFormDialog piutang/hutang', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
  })
  afterEach(() => {
    document.body.innerHTML = ''
  })

  async function pickKind(label: string) {
    key($('[data-testid="kind"]')!, 'Enter')
    await flushPromises()
    key([...document.body.querySelectorAll('[role="option"]')].find((o) => o.textContent?.trim() === label)!, 'Enter')
    await flushPromises()
  }

  it('receivable: kontak + arah "Saya berutang" mengirim saldo awal negatif', async () => {
    await ipc.createAccount('receivable', {
      name: 'Piutang Sari', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 0, openingDate: '2026-10-01', contact: 'Sari',
    })
    const store = useAccountsStore()
    const spy = vi.spyOn(store, 'createAccount').mockResolvedValue('x')
    await open()
    expect($('#acc-contact')).toBeNull()
    await pickKind(id.accountKinds.receivable)
    expect($('#acc-contact')).not.toBeNull()
    const options = [...document.body.querySelectorAll('datalist option')].map((o) => (o as HTMLOptionElement).value)
    expect(options).toContain('Sari')
    ;($('[data-testid="direction-iOwe"]') as HTMLElement).dispatchEvent(new MouseEvent('mousedown', { bubbles: true, button: 0 }))
    await flushPromises()
    await type('#acc-name', 'Hutang Budi')
    await type('#acc-contact', 'Budi')
    await type('#acc-opening', '50000')
    ;($('form') as HTMLFormElement).dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await flushPromises()
    expect(spy.mock.calls[0]![0]).toBe('receivable')
    expect(spy.mock.calls[0]![1]).toMatchObject({ openingBalance: -50_000, contact: 'Budi' })
  })

  it('edit hutang memilih arah "Saya berutang" dan saldo positif', async () => {
    await open({ account: { ...account('receivable'), openingBalance: -50_000, contact: 'Budi' } })
    expect($('[data-testid="direction-iOwe"]')?.getAttribute('data-state')).toBe('active')
    expect(($('#acc-opening') as HTMLInputElement).value).toBe('50000')
    expect(($('#acc-contact') as HTMLInputElement).value).toBe('Budi')
  })
})
