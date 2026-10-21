import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import type { AccountDto } from '@/ipc'
import AccountList from './AccountList.vue'

const base: AccountDto = {
  id: 'a', name: 'Akun', kind: 'bank', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
  icon: null, color: null, sortOrder: 0, archived: false, balance: 0, openingBalance: 0, openingDate: null,
  contact: null, reconciledThrough: null,
}
const key = (el: Element, k: string) => el.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true }))

async function list(accounts: AccountDto[]) {
  const w = mount(AccountList, { props: { groupId: null, accounts }, global: { plugins: [i18n] }, attachTo: document.body })
  await flushPromises()
  return w
}
const row = (accId: string) => document.body.querySelector(`[data-id="${accId}"]`)!

async function openMenu(accId: string) {
  key(row(accId).querySelector(`[aria-label="${id.actions.more}"]`)!, 'Enter')
  await flushPromises()
}

describe('AccountList', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
  })
  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('piutang/hutang: label arah, nominal tanpa minus, kontak sebagai meta', async () => {
    await list([
      { ...base, id: 'h', kind: 'receivable', balance: -50_000, contact: 'Budi' },
      { ...base, id: 'p', kind: 'receivable', balance: 20_000 },
      { ...base, id: 'l', kind: 'receivable', balance: 0 },
    ])
    const h = row('h').textContent!
    expect(h).toContain(id.accounts.payableLabel)
    expect(h).toContain('Budi')
    expect(h).toContain('50.000')
    expect(h).not.toContain('−')
    expect(row('p').textContent).toContain(id.accounts.receivableLabel)
    expect(row('l').textContent).toContain(id.accounts.settled)
  })

  it('direkonsiliasi s.d. tampil sebagai meta', async () => {
    await list([{ ...base, reconciledThrough: '2026-10-10' }])
    expect(row('a').textContent).toContain('Direkonsiliasi s.d. 10 Okt 2026')
  })

  it('menu Rekonsiliasi ada untuk bank, tidak untuk investasi', async () => {
    const w = await list([base, { ...base, id: 'inv', kind: 'investment' }])
    await openMenu('a')
    const item = document.body.querySelector('[data-testid="account-reconcile"]') as HTMLElement
    expect(item).not.toBeNull()
    item.click()
    await flushPromises()
    expect(w.emitted('reconcile')?.[0]?.[0]).toMatchObject({ id: 'a' })
    await openMenu('inv')
    expect(document.body.querySelector('[data-testid="account-reconcile"]')).toBeNull()
  })
})
