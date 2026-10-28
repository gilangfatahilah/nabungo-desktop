import { flushPromises, mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import type { EmailStatusDto } from '@/ipc'
import type { EmailSyncHandlers } from '@/ipc/browser-email'
import { createMemoryHistory, createRouter } from 'vue-router'
import { routes } from '@/router'
import EmailSection from './EmailSection.vue'

const ADDR = 'me@gmail.com'
const $ = (sel: string) => document.body.querySelector(sel) as HTMLElement | null
const button = (text: string) =>
  [...document.body.querySelectorAll('button')].find((b) => b.textContent?.trim() === text) as HTMLButtonElement | undefined

const disconnected: EmailStatusDto = {
  enabled: false, address: null, mailbox: 'Finance', lastSyncAt: null, running: false, lastError: null, paused: false,
}
const connected: EmailStatusDto = { ...disconnected, enabled: true, address: ADDR }

async function type(sel: string, value: string) {
  const el = $(sel) as HTMLInputElement
  el.value = value
  el.dispatchEvent(new Event('input', { bubbles: true }))
  await nextTick()
}

let handlers: EmailSyncHandlers = {}

async function mountSection(status: EmailStatusDto) {
  vi.spyOn(ipc, 'emailStatus').mockResolvedValue(status)
  vi.spyOn(ipc, 'onEmailSync').mockImplementation(async (h) => {
    handlers = h
    return () => {}
  })
  const router = createRouter({ history: createMemoryHistory(), routes })
  const w = mount(EmailSection, { global: { plugins: [i18n, router] }, attachTo: document.body })
  await flushPromises()
  return w
}

describe('EmailSection', () => {
  beforeEach(() => setActivePinia(createPinia()))
  afterEach(() => {
    vi.restoreAllMocks()
    handlers = {}
    document.body.innerHTML = ''
  })

  it('belum terhubung → form setup dengan langkah & label Finance', async () => {
    const w = await mountSection(disconnected)
    expect(w.text()).toContain(id.email.step2)
    expect(($('#email-mailbox') as HTMLInputElement).value).toBe('Finance')
    const pw = $('#email-password') as HTMLInputElement
    expect([pw.type, pw.getAttribute('autocomplete')]).toEqual(['password', 'off'])
    expect(button(id.email.connect)).toBeTruthy()
    w.unmount()
  })

  it('hubungkan sukses → status terhubung, password dikosongkan', async () => {
    const w = await mountSection(disconnected)
    const spy = vi.spyOn(ipc, 'connectEmail').mockResolvedValue()
    await type('#email-address', ADDR)
    await type('#email-password', 'abcd efgh')
    vi.mocked(ipc.emailStatus).mockResolvedValue(connected)
    button(id.email.connect)!.click()
    await flushPromises()
    expect(spy).toHaveBeenCalledWith(ADDR, 'Finance', 'abcd efgh')
    expect(w.text()).toContain(ADDR)
    expect(w.text()).toContain(id.email.neverSynced)
    expect(button(id.email.syncNow)).toBeTruthy()
    button(id.email.edit)!.click()
    await nextTick()
    expect(($('#email-password') as HTMLInputElement).value).toBe('')
    expect(($('#email-address') as HTMLInputElement).value).toBe(ADDR)
    w.unmount()
  })

  it('login gagal → pesan terjemahan, form tetap, password dikosongkan', async () => {
    const w = await mountSection(disconnected)
    vi.spyOn(ipc, 'connectEmail').mockRejectedValue(new ipc.IpcFailure({ code: 'email_auth_failed', entity: null }))
    await type('#email-address', ADDR)
    await type('#email-password', 'wrong')
    button(id.email.connect)!.click()
    await flushPromises()
    expect(w.text()).toContain(id.errors.email_auth_failed)
    expect(($('#email-address') as HTMLInputElement).value).toBe(ADDR)
    expect(($('#email-password') as HTMLInputElement).value).toBe('')
    w.unmount()
  })

  it('password wajib saat pertama terhubung', async () => {
    const w = await mountSection(disconnected)
    const spy = vi.spyOn(ipc, 'connectEmail').mockResolvedValue()
    await type('#email-address', ADDR)
    button(id.email.connect)!.click()
    await flushPromises()
    expect(spy).not.toHaveBeenCalled()
    expect(w.text()).toContain(id.email.passwordRequired)
    w.unmount()
  })

  it('sync sekarang nonaktif selama berjalan, aktif lagi saat done', async () => {
    const w = await mountSection(connected)
    let resolve!: (v: ipc.SyncSummaryDto) => void
    vi.spyOn(ipc, 'syncEmail').mockReturnValue(new Promise((r) => (resolve = r)))
    button(id.email.syncNow)!.click()
    await flushPromises()
    expect(button(id.email.syncNow)!.disabled).toBe(true)
    handlers.progress?.({ processed: 3, newItems: 2, unparsed: 1 })
    await nextTick()
    expect(w.text()).toContain(i18n.global.t('email.syncing', { n: 3 }))
    const summary = { newItems: 2, unparsed: 1, seen: 3 }
    vi.mocked(ipc.emailStatus).mockResolvedValue({ ...connected, lastSyncAt: '2026-10-28T03:00:00Z' })
    handlers.done?.({ summary, error: null })
    resolve(summary)
    await flushPromises()
    expect(button(id.email.syncNow)!.disabled).toBe(false)
    expect(w.text()).not.toContain(id.email.neverSynced)
    w.unmount()
  })

  it('terhubung → link Tinjau di Inbox', async () => {
    await mountSection(connected)
    const link = [...document.body.querySelectorAll('a')].find((a) => a.textContent?.trim() === id.email.reviewInInbox)!
    expect(link.getAttribute('href')).toContain('/inbox')
  })

  it('terhubung → link Kelola rule ke tab Rule', async () => {
    await mountSection(connected)
    const link = [...document.body.querySelectorAll('a')].find((a) => a.textContent?.trim() === id.emailRules.manageRules)!
    expect(link.getAttribute('href')).toContain('/inbox?tab=rules')
  })

  it('status dijeda → badge dan pesan galat terakhir', async () => {
    const w = await mountSection({ ...connected, paused: true, lastError: 'email_auth_failed' })
    expect(w.text()).toContain(id.email.paused)
    expect(w.text()).toContain(id.errors.email_auth_failed)
    w.unmount()
  })

  it('putuskan minta konfirmasi lalu kembali ke form dengan alamat terisi', async () => {
    const w = await mountSection(connected)
    const spy = vi.spyOn(ipc, 'disconnectEmail').mockResolvedValue()
    button(id.email.disconnect)!.click()
    await flushPromises()
    expect(document.body.textContent).toContain(id.email.disconnectBody)
    expect(spy).not.toHaveBeenCalled()
    vi.mocked(ipc.emailStatus).mockResolvedValue({ ...connected, enabled: false })
    const confirm = [...document.body.querySelectorAll('[role="alertdialog"] button')].find(
      (b) => b.textContent?.trim() === id.email.disconnect,
    ) as HTMLButtonElement
    confirm.click()
    await flushPromises()
    expect(spy).toHaveBeenCalled()
    expect(button(id.email.connect)).toBeTruthy()
    expect(($('#email-address') as HTMLInputElement).value).toBe(ADDR)
    w.unmount()
  })
})
