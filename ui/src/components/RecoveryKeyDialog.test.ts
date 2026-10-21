import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import { useSecurityStore } from '@/stores/security'
import RecoveryKeyDialog from './RecoveryKeyDialog.vue'

const KEY = 'ABCD-EFGH-JKMN-PQRS-TVWX-YZ01-2345-6789-ABCD-EFGH-JKMN-PQRS-TVWX'
const button = (text: string) =>
  [...document.body.querySelectorAll('button')].find((b) => b.textContent?.trim() === text) as HTMLButtonElement

async function open() {
  const w = mount(RecoveryKeyDialog, {
    props: { open: true, recoveryKey: KEY },
    global: { plugins: [i18n] },
    attachTo: document.body,
  })
  await flushPromises()
  return w
}

describe('RecoveryKeyDialog', () => {
  beforeEach(() => setActivePinia(createPinia()))
  afterEach(() => {
    document.body.innerHTML = ''
    vi.unstubAllGlobals()
  })

  it('menampilkan kunci per kelompok dan teks cetak', async () => {
    await open()
    const groups = [...document.body.querySelectorAll('[data-testid="recovery-group"]')].map((e) => e.textContent)
    expect(groups.join('-')).toBe(KEY)
    const print = document.body.querySelector('.print-recovery')!
    expect(print.textContent).toContain(id.security.recoveryDialog.printTitle)
    expect(print.textContent).toContain(KEY)
    expect(print.textContent).toContain(id.security.warning)
  })

  it('Selesai nonaktif sampai checkbox dicentang', async () => {
    useSecurityStore().recoveryPending = true
    const w = await open()
    expect(button(id.security.recoveryDialog.done).disabled).toBe(true)
    ;(document.body.querySelector('#recovery-saved') as HTMLElement).click()
    await flushPromises()
    expect(button(id.security.recoveryDialog.done).disabled).toBe(false)
    button(id.security.recoveryDialog.done).click()
    await flushPromises()
    expect(w.emitted('done')).toHaveLength(1)
    expect(useSecurityStore().recoveryPending).toBe(false)
    expect(w.emitted('update:open')!.at(-1)).toEqual([false])
  })

  it('Esc tidak menutup dialog', async () => {
    const w = await open()
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }))
    await flushPromises()
    expect(w.emitted('update:open')).toBeUndefined()
  })

  it('Salin menulis kunci ke clipboard', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    vi.stubGlobal('navigator', { ...navigator, clipboard: { writeText } })
    await open()
    button(id.security.recoveryDialog.copy).click()
    await flushPromises()
    expect(writeText).toHaveBeenCalledWith(KEY)
  })

  it('Cetak memanggil window.print', async () => {
    const print = vi.spyOn(window, 'print').mockImplementation(() => {})
    await open()
    button(id.security.recoveryDialog.print).click()
    expect(print).toHaveBeenCalledTimes(1)
  })
})
