import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
import { defineComponent, h } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'
import AppSidebar from '@/components/AppSidebar.vue'
import { SidebarProvider } from '@/components/ui/sidebar'
import { i18n } from '@/i18n'
import { routes } from '@/router'
import { useAccountsStore } from '@/stores/accounts'

beforeEach(() => setActivePinia(createPinia()))

async function mountSidebar(path: string) {
  const router = createRouter({ history: createMemoryHistory(), routes })
  await router.push(path)
  const store = useAccountsStore()
  store.overview = {
    groups: [],
    accounts: [{ id: 'bca', name: 'BCA', kind: 'bank', groupId: null, includeInNetWorth: true, creditLimit: null,
      statementDay: null, icon: null, color: null, sortOrder: 0, archived: false, balance: 4_343_000,
      openingBalance: 4_343_000, openingDate: '2026-10-01', contact: null, reconciledThrough: null }],
    netWorth: { assets: 4_343_000, liabilities: 0, net: 4_343_000 },
  }
  store.loaded = true
  const Host = defineComponent(() => () => h(SidebarProvider, null, () => h(AppSidebar)))
  const w = mount(Host, { global: { plugins: [i18n, router] } })
  await flushPromises()
  return w
}

describe('AppSidebar', () => {
  it('angka di sidebar tanpa prefiks Rp', async () => {
    const w = await mountSidebar('/')
    expect(w.find('[data-testid="sidebar-net-worth"]').text()).toBe('4.343.000')
    expect(w.find('[data-testid="sidebar-accounts"]').text()).not.toContain('Rp')
  })

  it('tombol utama berlabel "Transaksi baru"', async () => {
    const w = await mountSidebar('/')
    expect(w.find('[data-testid="new-txn"]').text()).toContain('Transaksi baru')
  })

  it('item aktif ditandai aria-current', async () => {
    const w = await mountSidebar('/transactions')
    expect(w.find('a[aria-current="page"]').text()).toContain('Transaksi')
  })

  it('header memakai wordmark dengan tanda koin', async () => {
    const w = await mountSidebar('/')
    const logo = w.find('[data-testid="app-logo"]')
    expect(logo.text()).toBe('Nabungo')
    expect(logo.find('svg[aria-hidden="true"]').exists()).toBe(true)
  })
})
