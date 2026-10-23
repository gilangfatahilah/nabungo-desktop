import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeAll, expect, it } from 'vitest'
import { nextTick } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'
import { i18n } from '@/i18n'
import * as ipc from '@/ipc'
import { routes } from '@/router'
import DashboardView from './DashboardView.vue'

// File terpisah agar browser-ledger baru: aset & utang terkendali.
const account = (name: string, openingBalance: number) => ({
  name, groupId: null, includeInNetWorth: true, creditLimit: null, statementDay: null,
  icon: null, color: null, openingBalance, openingDate: '2026-09-01', contact: null,
})

beforeAll(async () => {
  await ipc.completeOnboarding({ locale: 'id', periodStartDay: 1, accountKind: 'bank', account: account('BCA', 3_000_000) })
  await ipc.createAccount('cash', account('Dompet', 1_000_000))
  await ipc.createAccount('credit_card', account('Kartu', -500_000))
})

it('komposisi kekayaan: donut aset per jenis akun dan ringkasan aset/utang/bersih', async () => {
  setActivePinia(createPinia())
  i18n.global.locale.value = 'id'
  const router = createRouter({ history: createMemoryHistory(), routes })
  await router.push('/')
  mount(DashboardView, { global: { plugins: [i18n, router] }, attachTo: document.body })
  await flushPromises()
  await nextTick()
  await flushPromises()
  const card = document.body.querySelector<HTMLElement>('[data-testid="dashboard-networth"]')!
  expect(card).not.toBeNull()
  expect([...card.querySelectorAll('svg path[data-key]')].map((p) => p.getAttribute('data-key'))).toEqual(['cash', 'bank'])
  const rows = [...card.querySelectorAll<HTMLElement>('[data-testid="networth-row"]')]
    .map((r) => [...r.children].map((c) => c.textContent!.trim()).filter(Boolean))
  expect(rows).toEqual([['Tunai', '1.000.000', '25%'], ['Bank', '3.000.000', '75%']])
  const summary = [...card.querySelectorAll('[data-testid="networth-summary"] > div')]
    .map((d) => [...d.children].map((c) => c.textContent!.trim()))
  const legend = card.querySelectorAll<HTMLElement>('[data-testid="networth-row"]')[1]!
  legend.dispatchEvent(new MouseEvent('mouseenter'))
  await nextTick()
  expect(card.querySelector('path[data-key="cash"]')!.getAttribute('opacity')).toBe('0.4')
  legend.dispatchEvent(new MouseEvent('mouseleave'))
  card.querySelector('path[data-key="bank"]')!.dispatchEvent(new MouseEvent('mouseenter'))
  await nextTick()
  expect(card.querySelector('[data-part="tooltip"]')!.textContent).toMatch(/Bank.*Rp\s3\.000\.000.*75%/)
  expect(summary).toEqual([['Aset', '4.000.000'], ['Liabilitas', '500.000'], ['Kekayaan bersih', '3.500.000']])
})
