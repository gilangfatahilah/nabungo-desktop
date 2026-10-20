import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import { useSecurityStore } from '@/stores/security'
import UnlockView from './UnlockView.vue'

vi.mock('@/lib/reload', () => ({ reloadApp: vi.fn() }))

const PW = 'rahasia-123'
const blank = { template: '<div />' }

function makeRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', name: 'dashboard', component: blank },
      { path: '/accounts', name: 'accounts', component: blank },
      { path: '/unlock', name: 'unlock', component: blank },
    ],
  })
}

async function lockedApp() {
  await ipc.enableEncryption(PW)
  await ipc.lock()
}

async function cleanup() {
  await ipc.unlock(PW)
  await ipc.disableEncryption(PW)
}

describe('UnlockView', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
  })

  it('password salah menampilkan pesan', async () => {
    await lockedApp()
    const router = makeRouter()
    await router.push('/unlock')
    const w = mount(UnlockView, { global: { plugins: [i18n, router] } })
    await flushPromises()
    await w.find('#unlock-password').setValue('salah-1234')
    await w.find('form').trigger('submit')
    await flushPromises()
    expect(w.text()).toContain(id.errors.wrong_password)
    expect(router.currentRoute.value.name).toBe('unlock')
    await w.find('#unlock-password').setValue(PW)
    await w.find('form').trigger('submit')
    await flushPromises()
    expect(router.currentRoute.value.name).toBe('dashboard')
    await ipc.disableEncryption(PW)
  })

  it('mode recovery memvalidasi konfirmasi password', async () => {
    await lockedApp()
    const spy = vi.spyOn(ipc, 'unlockWithRecovery')
    const router = makeRouter()
    await router.push('/unlock')
    const w = mount(UnlockView, { global: { plugins: [i18n, router] } })
    await flushPromises()
    await w.get('[data-testid="unlock-use-recovery"]').trigger('click')
    await w.find('#unlock-recovery').setValue('AAAA-BBBB')
    await w.find('#unlock-new-password').setValue('baru-12345')
    await w.find('#unlock-confirm-password').setValue('beda-12345')
    await w.find('form').trigger('submit')
    await flushPromises()
    expect(w.text()).toContain(id.security.passwordMismatch)
    expect(spy).not.toHaveBeenCalled()
    spy.mockRestore()
    await cleanup()
  })

  it('redirects to unlock on locked error', async () => {
    const router = makeRouter()
    await router.push('/accounts')
    const security = useSecurityStore()
    security.installLockHandler(router)
    await lockedApp()
    await expect(ipc.accountsOverview()).rejects.toBeInstanceOf(ipc.IpcFailure)
    await flushPromises()
    expect(router.currentRoute.value.name).toBe('unlock')
    expect(router.currentRoute.value.query.redirect).toBe('/accounts')
    expect(security.locked).toBe(true)
    await cleanup()
  })
})
