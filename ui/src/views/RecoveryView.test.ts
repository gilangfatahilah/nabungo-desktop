import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import { __enterRecovery, __leaveRecovery } from '@/ipc/browser-ledger'
import RecoveryView from './RecoveryView.vue'

vi.mock('@/lib/reload', () => ({ reloadApp: vi.fn() }))

const blank = { template: '<div />' }
const button = (text: string, root: ParentNode = document.body) =>
  [...root.querySelectorAll('button')].find((b) => b.textContent?.trim() === text) as HTMLButtonElement
const dialog = () => document.body.querySelector('[role="alertdialog"]') as HTMLElement
const rows = () => document.body.querySelectorAll('[data-testid="recovery-backup-row"]')

async function mountView() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', name: 'dashboard', component: blank },
      { path: '/onboarding', name: 'onboarding', component: blank },
      { path: '/unlock', name: 'unlock', component: blank },
      { path: '/recovery', name: 'recovery', component: blank },
    ],
  })
  await router.push('/recovery')
  const w = mount(RecoveryView, { global: { plugins: [i18n, router] }, attachTo: document.body })
  await flushPromises()
  return { w, router }
}

// Urutan penting: browser-ledger adalah singleton per file test.
describe('RecoveryView', () => {
  beforeEach(async () => {
    setActivePinia(createPinia())
    i18n.global.locale.value = 'id'
  })
  afterEach(() => {
    __leaveRecovery()
    document.body.innerHTML = ''
  })

  it('shows_reason_and_backups', async () => {
    await ipc.backupNow()
    __enterRecovery('unreadable')
    const { w } = await mountView()
    expect(w.find('h1').text()).toBe(id.recovery.titleUnreadable)
    expect(rows()).toHaveLength(1)
    expect(document.body.textContent).not.toContain(id.recovery.tooNewWarning)
    w.unmount()
  })

  it('too_new_shows_update_warning', async () => {
    __enterRecovery('tooNew')
    const { w } = await mountView()
    expect(w.find('h1').text()).toBe(id.recovery.titleTooNew)
    expect(document.body.textContent).toContain(id.recovery.tooNewWarning)
    w.unmount()
  })

  it('restore_named_backup_leaves_recovery', async () => {
    __enterRecovery('unreadable')
    const { w, router } = await mountView()
    button(id.recovery.restore, rows()[0]!).click()
    await flushPromises()
    expect(dialog().textContent).toContain(id.recovery.confirmTitle)
    button(id.recovery.confirm, dialog()).click()
    await flushPromises()
    expect((await ipc.recoveryStatus()).active).toBe(false)
    expect(router.currentRoute.value.name).toBe('dashboard')
    w.unmount()
  })

  it('backup terenkripsi: error recovery key menempel di field', async () => {
    vi.spyOn(ipc, 'inspectBackupSource').mockResolvedValueOnce({ needsKey: true })
    const recover = vi.spyOn(ipc, 'recoverFromBackup').mockRejectedValueOnce(new ipc.IpcFailure({ code: 'invalid_recovery_key', entity: null }))
    __enterRecovery('unreadable')
    const { w } = await mountView()
    button(id.recovery.restore, rows()[0]!).click()
    await flushPromises()
    // Tanpa key file hanya recovery key + password baru.
    expect(w.find('#recovery-password').exists()).toBe(false)
    await w.find('#recovery-key').setValue('AAAA-BBBB')
    await w.find('#recovery-new-password').setValue('baru-12345')
    await w.find('#recovery-confirm-password').setValue('baru-12345')
    await w.find('form[data-testid="recovery-secret-form"]').trigger('submit')
    await flushPromises()
    button(id.recovery.confirm, dialog()).click()
    await flushPromises()
    expect(recover).toHaveBeenCalledWith(
      expect.objectContaining({ kind: 'named' }),
      { kind: 'recoveryKey', recoveryKey: 'AAAA-BBBB', newPassword: 'baru-12345' },
    )
    expect(w.find('#recovery-key').attributes('aria-invalid')).toBe('true')
    expect(w.find('#recovery-error').text()).toBe(id.errors.invalid_recovery_key)
    recover.mockRestore()
    w.unmount()
  })

  // Terakhir: data browser dikosongkan.
  it('start_fresh_goes_to_onboarding', async () => {
    __enterRecovery('unreadable')
    const { w, router } = await mountView()
    button(id.recovery.fresh).click()
    await flushPromises()
    button(id.recovery.freshConfirm, dialog()).click()
    await flushPromises()
    expect((await ipc.recoveryStatus()).active).toBe(false)
    expect(await ipc.getOnboardingStatus()).toBe(false)
    expect(router.currentRoute.value.name).toBe('onboarding')
    w.unmount()
  })
})
