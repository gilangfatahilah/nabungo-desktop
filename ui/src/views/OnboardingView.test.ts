import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'
import { i18n } from '@/i18n'
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
})
