import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import type { AccountDto } from '@/ipc'
import { useAccountsStore } from '@/stores/accounts'
import AccountFormDialog from './AccountFormDialog.vue'

function account(kind: AccountDto['kind']): AccountDto {
  return {
    id: 'a1', name: 'Kartu', kind, groupId: null, includeInNetWorth: true, creditLimit: 5_000_000, statementDay: 20,
    icon: null, color: null, sortOrder: 1, archived: false, balance: -100_000, openingBalance: 0, openingDate: null,
  }
}

// Dialog di-teleport ke body: query lewat document.
const $ = (sel: string) => document.body.querySelector(sel)

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
})
