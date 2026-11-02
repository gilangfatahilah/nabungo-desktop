import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { toast } from 'vue-sonner'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import { browserUpdate } from '@/ipc/browser-update'
import UpdateSection from './UpdateSection.vue'

const button = (text: string) =>
  [...document.body.querySelectorAll('button')].find((b) => b.textContent?.trim().startsWith(text)) as HTMLButtonElement | undefined
const $ = (sel: string) => document.body.querySelector(sel) as HTMLElement | null

async function mountSection() {
  const w = mount(UpdateSection, { global: { plugins: [i18n] }, attachTo: document.body })
  await flushPromises()
  return w
}

async function clickAndSettle(text: string) {
  await vi.waitFor(() => expect(button(text)).toBeTruthy())
  button(text)!.click()
  await flushPromises()
}

describe('UpdateSection', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    browserUpdate.seed({ latest: null, notes: '', kind: null, fail: null })
  })
  afterEach(() => {
    vi.restoreAllMocks()
    document.body.innerHTML = ''
  })

  it('versi saat ini dan cek otomatis mati secara default', async () => {
    const w = await mountSection()
    expect($('[data-testid=update-current]')!.textContent).toContain('0.4.0')
    expect($('#update-auto')!.getAttribute('aria-checked')).toBe('false')
    w.unmount()
  })

  it('tidak ada versi baru → sudah terbaru', async () => {
    const w = await mountSection()
    await clickAndSettle(id.update.check)
    expect($('[data-testid=update-uptodate]')).toBeTruthy()
    expect($('[data-testid=update-available]')).toBeNull()
    w.unmount()
  })

  it('versi baru (rpm) → catatan rilis, unduh, lalu installer dibuka', async () => {
    browserUpdate.seed({ latest: '0.5.0', notes: 'Fitur update', kind: 'rpm' })
    const install = vi.spyOn(ipc, 'installUpdate')
    const w = await mountSection()
    await clickAndSettle(id.update.check)
    const card = $('[data-testid=update-available]')!
    expect(card.textContent).toContain('0.5.0')
    expect(card.textContent).toContain('Fitur update')
    await clickAndSettle(id.update.download)
    await vi.waitFor(() => expect(install).toHaveBeenCalledOnce())
    w.unmount()
  })

  it('Windows meminta konfirmasi sebelum aplikasi ditutup', async () => {
    browserUpdate.seed({ latest: '0.5.0', kind: 'nsis' })
    const install = vi.spyOn(ipc, 'installUpdate')
    const w = await mountSection()
    await clickAndSettle(id.update.check)
    await clickAndSettle(id.update.download)
    await vi.waitFor(() => expect(document.body.textContent).toContain(id.update.confirmTitle))
    expect(install).not.toHaveBeenCalled()
    w.unmount()
  })

  it('format instalasi tidak dikenal → hanya halaman rilis', async () => {
    browserUpdate.seed({ latest: '0.5.0', kind: null })
    const w = await mountSection()
    await clickAndSettle(id.update.check)
    expect($('[data-testid=update-available]')!.textContent).toContain(id.update.manualOnly)
    expect(button(id.update.download)).toBeUndefined()
    expect(button(id.update.openReleasePage)).toBeTruthy()
    w.unmount()
  })

  it('galat jaringan → pesan errors.update_network', async () => {
    browserUpdate.seed({ fail: 'update_network' })
    const err = vi.spyOn(toast, 'error')
    const w = await mountSection()
    await clickAndSettle(id.update.check)
    expect(err).toHaveBeenCalledWith(id.errors.update_network)
    expect($('[data-testid=update-uptodate]')).toBeNull()
    w.unmount()
  })
})
