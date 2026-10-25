import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { toast } from 'vue-sonner'
import { createMemoryHistory, createRouter } from 'vue-router'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import { reloadApp } from '@/lib/reload'
import SettingsView from './SettingsView.vue'

vi.mock('@/lib/reload', () => ({ reloadApp: vi.fn() }))

const button = (text: string, root: ParentNode = document.body) =>
  [...root.querySelectorAll('button')].find((b) => b.textContent?.trim() === text) as HTMLElement
const rows = () => document.body.querySelectorAll('[data-testid="backup-row"]')

async function mountView() {
  const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/', component: { template: '<div />' } }] })
  const w = mount(SettingsView, { global: { plugins: [i18n, router] }, attachTo: document.body })
  await flushPromises()
  return w
}

// Urutan penting: browser-ledger adalah singleton per file test.
describe('SettingsView backup & ekspor', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers({ toFake: ['Date'] })
    vi.setSystemTime(new Date(2026, 9, 17))
  })
  afterEach(() => {
    vi.useRealTimers()
    vi.mocked(reloadApp).mockClear()
    document.body.innerHTML = ''
  })

  it('daftar kosong menampilkan empty state', async () => {
    const w = await mountView()
    expect(document.body.textContent).toContain(id.settings.backup.empty)
    expect(rows()).toHaveLength(0)
    w.unmount()
  })

  it('backup sekarang menambah baris di daftar', async () => {
    const w = await mountView()
    button(id.settings.backup.backupNow).click()
    await flushPromises()
    expect(rows()).toHaveLength(1)
    expect(rows()[0]!.textContent).toContain(id.settings.backup.kinds.auto)
    w.unmount()
  })

  it('pulihkan meminta konfirmasi lalu memuat ulang', async () => {
    const w = await mountView()
    const restore = () => rows()[0]!.querySelector('button') as HTMLElement

    restore().click()
    await flushPromises()
    expect(document.body.querySelector('[role="alertdialog"]')?.textContent).toContain('17 Okt 2026')
    button(id.actions.cancel).click()
    await flushPromises()
    expect(reloadApp).not.toHaveBeenCalled()

    restore().click()
    await flushPromises()
    button(id.settings.backup.restoreConfirm, document.body.querySelector('[role="alertdialog"]')!).click()
    await flushPromises()
    expect(reloadApp).toHaveBeenCalledTimes(1)
    w.unmount()
  })

  it('ekspor JSON menampilkan toast path', async () => {
    const success = vi.spyOn(toast, 'success')
    const w = await mountView()
    button(id.settings.export.json).click()
    await flushPromises()
    expect(success).toHaveBeenCalledWith(expect.stringContaining('browser://export.json'))
    w.unmount()
  })
})

describe('SettingsView ukuran teks', () => {
  beforeEach(() => setActivePinia(createPinia()))
  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('menampilkan pilihan ukuran teks berlabel', async () => {
    await mountView()
    const label = [...document.body.querySelectorAll('label')].find((l) => l.textContent === id.settings.fontScale)
    expect(label).toBeTruthy()
    const trigger = document.body.querySelector(`[aria-labelledby="${label!.id}"]`) as HTMLElement
    expect(trigger.textContent).toContain('100%')
    expect(document.body.textContent).toContain(id.settings.fontScaleHint)
  })
})
