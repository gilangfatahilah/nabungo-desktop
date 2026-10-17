import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import AccountList from '@/components/AccountList.vue'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import { IpcFailure, type AccountDto } from '@/ipc'
import { useAccountsStore } from '@/stores/accounts'
import AccountsView from './AccountsView.vue'

const bca: AccountDto = {
  id: 'bca', name: 'BCA', kind: 'bank', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
  icon: null, color: null, sortOrder: 0, archived: false, balance: 50_000, openingBalance: 50_000, openingDate: '2026-10-01',
}

function clickButton(text: string) {
  const b = [...document.body.querySelectorAll('button')].find((el) => el.textContent?.trim() === text)
  if (!b) throw new Error(`button "${text}" tidak ada`)
  b.click()
}

describe('AccountsView', () => {
  beforeEach(() => setActivePinia(createPinia()))
  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('hapus akun bertransaksi menawarkan arsip', async () => {
    const store = useAccountsStore()
    store.overview = { groups: [], accounts: [bca], netWorth: { assets: 50_000, liabilities: 0, net: 50_000 } }
    store.loaded = true
    vi.spyOn(store, 'load').mockResolvedValue()
    const del = vi.spyOn(store, 'deleteAccount').mockRejectedValue(new IpcFailure({ code: 'in_use', entity: 'account' }))
    const archive = vi.spyOn(store, 'archiveAccount').mockResolvedValue()

    const w = mount(AccountsView, { global: { plugins: [i18n] }, attachTo: document.body })
    await flushPromises()
    w.findComponent(AccountList).vm.$emit('delete', bca)
    await flushPromises()
    expect(document.body.textContent).toContain(id.accounts.deleteTitle)

    clickButton(id.actions.delete)
    await flushPromises()
    expect(del).toHaveBeenCalledWith('bca')
    expect(document.body.textContent).toContain(id.accounts.inUseBody)

    clickButton(id.actions.archive)
    await flushPromises()
    expect(archive).toHaveBeenCalledWith('bca')
    w.unmount()
  })

  it('grup tampil sebagai header baris dengan subtotal; akun terarsip terlipat', async () => {
    const store = useAccountsStore()
    const lama: AccountDto = { ...bca, id: 'lama', name: 'Rekening lama', groupId: null, archived: true, balance: 0 }
    store.overview = {
      groups: [{ id: 'g1', name: 'Bank', sortOrder: 0 }],
      accounts: [{ ...bca, groupId: 'g1' }, lama],
      netWorth: { assets: 50_000, liabilities: 0, net: 50_000 },
    }
    store.loaded = true
    vi.spyOn(store, 'load').mockResolvedValue()

    const w = mount(AccountsView, { global: { plugins: [i18n] }, attachTo: document.body })
    await flushPromises()
    const header = w.get('[data-testid="account-group"]')
    expect(header.text()).toContain('Bank')
    expect(header.get('[data-testid="group-subtotal"]').text()).toMatch(/50\.000$/)
    const row = w.get('[data-testid="account-row"]')
    expect(row.text()).toContain('BCA')
    expect(row.text()).toContain(id.accountKinds.bank)

    const toggle = w.get('[data-testid="archived-toggle"]')
    expect(toggle.text()).toContain(`${id.accounts.archived} (1)`)
    expect(w.text()).not.toContain('Rekening lama')
    await toggle.trigger('click')
    expect(w.text()).toContain('Rekening lama')
    w.unmount()
  })
})
