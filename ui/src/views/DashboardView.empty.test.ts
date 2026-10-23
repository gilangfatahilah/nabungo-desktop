import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeAll, expect, it } from 'vitest'
import { nextTick } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'
import { i18n } from '@/i18n'
import * as ipc from '@/ipc'
import { routes } from '@/router'
import DashboardView from './DashboardView.vue'

// File terpisah agar browser-ledger baru: hanya onboarding, tanpa transaksi, jadwal, atau bucket.
beforeAll(async () => {
  await ipc.completeOnboarding({
    locale: 'id', periodStartDay: 1, accountKind: 'bank',
    account: { name: 'BCA', groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
      icon: null, color: null, openingBalance: 0, openingDate: '2026-09-01', contact: null },
  })
})

it('kartu baru tersembunyi tanpa data', async () => {
  setActivePinia(createPinia())
  const router = createRouter({ history: createMemoryHistory(), routes })
  await router.push('/')
  mount(DashboardView, { global: { plugins: [i18n, router] }, attachTo: document.body })
  await flushPromises()
  await nextTick()
  await flushPromises()
  expect(document.body.querySelector('[data-testid="kpi-income"]')).not.toBeNull()
  for (const id of ['dashboard-expense', 'budget-watch', 'dashboard-due', 'dashboard-allocation', 'dashboard-networth']) {
    expect(document.body.querySelector(`[data-testid="${id}"]`)).toBeNull()
  }
  // Tanpa budget: kartu Budget periode ini mengajak mengatur budget, bukan menghilang.
  const cta = document.body.querySelector<HTMLAnchorElement>('[data-testid="budget-total-empty"] a')!
  expect(cta.textContent).toContain('Atur budget')
  expect(cta.getAttribute('href')).toContain('/budget')
})
