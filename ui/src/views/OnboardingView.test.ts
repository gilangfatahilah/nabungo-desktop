import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import OnboardingView from './OnboardingView.vue'

describe('OnboardingView', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('saldo yang baru diketik ikut terkirim saat Enter tanpa blur', async () => {
    const spy = vi.spyOn(ipc, 'completeOnboarding').mockResolvedValue('acc-1')
    const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/', component: { template: '<div />' } }] })
    const w = mount(OnboardingView, { global: { plugins: [i18n, router] } })
    await w.find('#ob-name').setValue('Dompet')
    const bal = w.find('#ob-balance')
    ;(bal.element as HTMLInputElement).value = '1500000'
    await bal.trigger('input')
    await w.find('form').trigger('submit')
    await flushPromises()
    expect(spy).toHaveBeenCalledTimes(1)
    expect(spy.mock.calls[0]![0].account.openingBalance).toBe(1_500_000)
  })

  it('tanggal periode ditulis sebagai kalimat', async () => {
    const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/', component: { template: '<div />' } }] })
    const w = mount(OnboardingView, { global: { plugins: [i18n, router] } })
    expect(w.get('[data-testid="ob-period"]').text()).toBe(id.onboarding.periodDay.replace('{day}', '1'))
  })

  it('tombol tetap aktif; nama kosong memunculkan error di field', async () => {
    const spy = vi.spyOn(ipc, 'completeOnboarding').mockResolvedValue('acc-1')
    spy.mockClear()
    const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/', component: { template: '<div />' } }] })
    const w = mount(OnboardingView, { global: { plugins: [i18n, router] } })
    const submit = w.get('button[type="submit"]')
    expect(submit.attributes('disabled')).toBeUndefined()
    await w.find('form').trigger('submit')
    await flushPromises()
    expect(spy).not.toHaveBeenCalled()
    expect(w.get('#ob-name').attributes('aria-invalid')).toBe('true')
    expect(w.get('#ob-name-error').text()).toBe(id.errors.name_empty)

    await w.find('#ob-name').setValue('Dompet')
    expect(w.find('#ob-name-error').exists()).toBe(false)
  })
})
