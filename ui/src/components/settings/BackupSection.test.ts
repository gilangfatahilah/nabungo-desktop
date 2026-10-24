import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import { reloadApp } from '@/lib/reload'
import BackupSection from './BackupSection.vue'

vi.mock('@/lib/reload', () => ({ reloadApp: vi.fn() }))
vi.mock('@/ipc', async (orig) => {
  const actual = await orig<typeof import('@/ipc')>()
  return {
    ...actual,
    listBackups: vi.fn(async () => [
      { fileName: 'auto-20261017T000000Z.db', kind: 'auto', createdAt: '2026-10-17T00:00:00Z', sizeBytes: 4096 },
    ]),
    inspectBackupSource: vi.fn(async () => ({ needsKey: true })),
    pickBackupFile: vi.fn(async () => '/media/usb/nabungo-lama.db'),
    restoreBackup: vi.fn(async () => {}),
  }
})

const button = (text: string, root: ParentNode = document.body) =>
  [...root.querySelectorAll('button')].find((b) => b.textContent?.trim() === text) as HTMLButtonElement
const dialog = () => document.body.querySelector('[role="alertdialog"]') as HTMLElement
const keyInput = () => dialog().querySelector('#restore-recovery-key') as HTMLInputElement | null

async function mountSection() {
  const w = mount(BackupSection, { global: { plugins: [i18n] }, attachTo: document.body })
  await flushPromises()
  return w
}

async function typeKey(value: string) {
  const input = keyInput()!
  input.value = value
  input.dispatchEvent(new Event('input'))
  await flushPromises()
}

describe('BackupSection restore dengan recovery key', () => {
  beforeEach(() => setActivePinia(createPinia()))
  afterEach(() => {
    vi.mocked(ipc.restoreBackup).mockReset().mockResolvedValue(undefined)
    vi.mocked(ipc.inspectBackupSource).mockClear()
    vi.mocked(reloadApp).mockClear()
    document.body.innerHTML = ''
  })

  it('backup butuh key → field recovery key, key dikirim ke restoreBackup', async () => {
    const w = await mountSection()
    button(id.settings.backup.restore).click()
    await flushPromises()
    expect(ipc.inspectBackupSource).toHaveBeenCalledWith({ kind: 'named', fileName: 'auto-20261017T000000Z.db' })
    expect(keyInput()).not.toBeNull()
    expect(button(id.settings.backup.restoreConfirm, dialog()).disabled).toBe(true)

    await typeKey('ABCD-EFGH')
    button(id.settings.backup.restoreConfirm, dialog()).click()
    await flushPromises()
    expect(ipc.restoreBackup).toHaveBeenCalledWith({ kind: 'named', fileName: 'auto-20261017T000000Z.db' }, 'ABCD-EFGH')
    expect(reloadApp).toHaveBeenCalledOnce()
    w.unmount()
  })

  it('invalid_recovery_key menempel di field, dialog tetap terbuka', async () => {
    vi.mocked(ipc.restoreBackup).mockRejectedValueOnce(new ipc.IpcFailure({ code: 'invalid_recovery_key', entity: null }))
    const w = await mountSection()
    button(id.settings.backup.restore).click()
    await flushPromises()
    await typeKey('SALAH')
    button(id.settings.backup.restoreConfirm, dialog()).click()
    await flushPromises()
    expect(keyInput()!.getAttribute('aria-invalid')).toBe('true')
    expect(dialog().querySelector('#restore-recovery-key-error')?.textContent).toBe(id.errors.invalid_recovery_key)
    expect(reloadApp).not.toHaveBeenCalled()
    w.unmount()
  })

  it('pulihkan dari file: pilih file dulu, lalu konfirmasi dengan path', async () => {
    vi.mocked(ipc.inspectBackupSource).mockResolvedValueOnce({ needsKey: false })
    const w = await mountSection()
    button(id.settings.backup.restoreFromFile).click()
    await flushPromises()
    expect(ipc.inspectBackupSource).toHaveBeenCalledWith({ kind: 'path', path: '/media/usb/nabungo-lama.db' })
    expect(keyInput()).toBeNull()
    button(id.settings.backup.restoreConfirm, dialog()).click()
    await flushPromises()
    expect(ipc.restoreBackup).toHaveBeenCalledWith({ kind: 'path', path: '/media/usb/nabungo-lama.db' }, null)
    w.unmount()
  })

  it('pilih file dibatalkan → tidak ada dialog', async () => {
    vi.mocked(ipc.pickBackupFile).mockResolvedValueOnce(null)
    const w = await mountSection()
    button(id.settings.backup.restoreFromFile).click()
    await flushPromises()
    expect(ipc.inspectBackupSource).not.toHaveBeenCalled()
    expect(dialog()).toBeNull()
    w.unmount()
  })
})
