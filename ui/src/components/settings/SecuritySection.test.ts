import { flushPromises, mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import SecuritySection from './SecuritySection.vue'

vi.mock('@/lib/reload', () => ({ reloadApp: vi.fn() }))

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

async function mountSection() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/', name: 'dashboard', component: { template: '<div />' } },
      { path: '/unlock', name: 'unlock', component: { template: '<div />' } },
    ],
  })
  const w = mount(SecuritySection, { global: { plugins: [i18n, router] }, attachTo: document.body })
  await flushPromises()
  return w
}

// Urutan penting: browser-ledger adalah singleton per file test.
describe('SecuritySection', () => {
  beforeEach(() => setActivePinia(createPinia()))
  afterEach(() => {
    vi.restoreAllMocks()
    document.body.innerHTML = ''
  })

  it('menjelaskan bahwa lampiran tidak ikut terenkripsi', async () => {
    const w = await mountSection()
    expect(w.text()).toContain(id.security.attachmentsNote)
    w.unmount()
  })

  it('aktifkan → menampilkan recovery key dari ipc', async () => {
    const spy = vi.spyOn(ipc, 'enableEncryption').mockResolvedValue(KEY)
    const w = await mountSection()
    button(id.security.enable).click()
    await flushPromises()
    await type('#enc-password', PW)
    await type('#enc-confirm', PW)
    $('#enc-ack').click()
    await flushPromises()
    ;(document.body.querySelector('[data-testid="enable-form"]') as HTMLFormElement).requestSubmit()
    await flushPromises()
    expect(spy).toHaveBeenCalledWith(PW)
    const groups = [...document.body.querySelectorAll('[data-testid="recovery-group"]')].map((e) => e.textContent)
    expect(groups.join('-')).toBe(KEY)
    w.unmount()
  })

  it('aktifkan tanpa centang peringatan → error di field, ipc tidak dipanggil', async () => {
    const spy = vi.spyOn(ipc, 'enableEncryption')
    const w = await mountSection()
    button(id.security.enable).click()
    await flushPromises()
    await type('#enc-password', PW)
    await type('#enc-confirm', PW)
    ;(document.body.querySelector('[data-testid="enable-form"]') as HTMLFormElement).requestSubmit()
    await flushPromises()
    expect(spy).not.toHaveBeenCalled()
    expect(document.body.textContent).toContain(id.security.ackRequired)
    w.unmount()
  })

  it('terenkripsi → menampilkan kunci otomatis 5 menit', async () => {
    await ipc.enableEncryption(PW)
    const w = await mountSection()
    expect(document.body.textContent).toContain(id.security.encryptedBadge)
    expect($('[data-testid="auto-lock"]').textContent).toContain(id.security.minutes.replace('{n}', '5'))
    w.unmount()
  })

  it('ganti password salah → pesan wrong_password', async () => {
    const w = await mountSection()
    button(id.security.changePassword).click()
    await flushPromises()
    await type('#cp-old', 'bukan-password')
    await type('#cp-new', 'password-baru-1')
    await type('#cp-confirm', 'password-baru-1')
    ;(document.body.querySelector('[data-testid="change-password-form"]') as HTMLFormElement).requestSubmit()
    await flushPromises()
    expect($('#cp-old-error').textContent).toBe(id.errors.wrong_password)
    w.unmount()
  })

  it('matikan enkripsi dengan password → kembali tidak terenkripsi', async () => {
    const w = await mountSection()
    button(id.security.disable).click()
    await flushPromises()
    await type('#disable-password', PW)
    ;(document.body.querySelector('[data-testid="disable-form"]') as HTMLFormElement).requestSubmit()
    await flushPromises()
    expect((await ipc.securityStatus()).encrypted).toBe(false)
    expect(button(id.security.enable)).toBeTruthy()
    w.unmount()
  })
})
