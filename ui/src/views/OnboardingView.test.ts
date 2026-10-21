import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { toast } from 'vue-sonner'
import { createMemoryHistory, createRouter } from 'vue-router'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import OnboardingView from './OnboardingView.vue'

const PW = 'rahasia-123'
const KEY = 'ABCD-EFGH-JKMN-PQRS-TVWX-YZ01-2345-6789-ABCD-EFGH-JKMN-PQRS-TVWX'
const $ = (sel: string) => document.body.querySelector(sel) as HTMLElement
const button = (text: string) =>
  [...document.body.querySelectorAll('button')].find((b) => b.textContent?.trim() === text) as HTMLButtonElement

async function type(sel: string, value: string) {
  const el = $(sel) as HTMLInputElement
  el.value = value
  el.dispatchEvent(new Event('input', { bubbles: true }))
  await nextTick()
}

async function mountAtOnboarding() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', component: { template: '<div />' } },
      { path: '/onboarding', component: { template: '<div />' } },
    ],
  })
  await router.push('/onboarding')
  const w = mount(OnboardingView, { global: { plugins: [i18n, router] }, attachTo: document.body })
  return { w, router }
}

async function fillWithEncryption() {
  await type('#ob-name', 'Dompet')
  $('[data-testid="ob-encrypt"]').click()
  await flushPromises()
  await type('#enc-password', PW)
  await type('#enc-confirm', PW)
  $('#enc-ack').click()
  await flushPromises()
  ;(document.body.querySelector('form') as HTMLFormElement).requestSubmit()
  await flushPromises()
}

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

  describe('enkripsi opsional', () => {
    afterEach(() => {
      vi.restoreAllMocks()
      document.body.innerHTML = ''
    })

    it('enkripsi default mati', async () => {
      const { w } = await mountAtOnboarding()
      expect($('[data-testid="ob-encrypt"]').getAttribute('aria-checked')).toBe('false')
      expect($('#enc-password')).toBeNull()
      w.unmount()
    })

    it('menyalakan enkripsi → enableEncryption dipanggil setelah completeOnboarding dan dialog recovery key tampil', async () => {
      const complete = vi.spyOn(ipc, 'completeOnboarding').mockResolvedValue('acc-1')
      const enable = vi.spyOn(ipc, 'enableEncryption').mockResolvedValue(KEY)
      const { w, router } = await mountAtOnboarding()
      await fillWithEncryption()
      expect(complete).toHaveBeenCalledTimes(1)
      expect(enable).toHaveBeenCalledWith(PW)
      expect(complete.mock.invocationCallOrder[0]!).toBeLessThan(enable.mock.invocationCallOrder[0]!)
      const groups = [...document.body.querySelectorAll('[data-testid="recovery-group"]')].map((e) => e.textContent)
      expect(groups.join('-')).toBe(KEY)
      expect(router.currentRoute.value.path).toBe('/onboarding')

      $('#recovery-saved').click()
      await flushPromises()
      button(id.security.recoveryDialog.done).click()
      await flushPromises()
      expect(router.currentRoute.value.path).toBe('/')
      w.unmount()
    })

    it('enkripsi menyala tapi belum valid → error di field, onboarding tidak dikirim', async () => {
      const complete = vi.spyOn(ipc, 'completeOnboarding').mockResolvedValue('acc-1')
      const { w } = await mountAtOnboarding()
      await type('#ob-name', 'Dompet')
      $('[data-testid="ob-encrypt"]').click()
      await flushPromises()
      ;(document.body.querySelector('form') as HTMLFormElement).requestSubmit()
      await flushPromises()
      expect(complete).not.toHaveBeenCalled()
      expect($('#enc-password').getAttribute('aria-invalid')).toBe('true')
      w.unmount()
    })

    it('enableEncryption gagal → onboarding tetap selesai dengan toast', async () => {
      vi.spyOn(ipc, 'completeOnboarding').mockResolvedValue('acc-1')
      vi.spyOn(ipc, 'enableEncryption').mockRejectedValue(new ipc.IpcFailure({ code: 'internal', entity: null }))
      const err = vi.spyOn(toast, 'error')
      const { w, router } = await mountAtOnboarding()
      await fillWithEncryption()
      expect(err).toHaveBeenCalledTimes(1)
      expect(err.mock.calls[0]![1]).toEqual({ description: id.onboarding.encryptLater })
      expect(router.currentRoute.value.path).toBe('/')
      w.unmount()
    })
  })
})
