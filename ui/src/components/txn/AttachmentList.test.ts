import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import AttachmentList from '@/components/txn/AttachmentList.vue'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import * as ipc from '@/ipc'
import { IpcFailure, type AttachmentDto } from '@/ipc'
import { useTransactionsStore } from '@/stores/transactions'

const png: AttachmentDto = { id: 'a1', fileName: 'struk.png', mime: 'image/png', sizeBytes: 1229, missing: false }
const pdf: AttachmentDto = { id: 'a2', fileName: 'nota.pdf', mime: 'application/pdf', sizeBytes: 2_621_440, missing: true }

const button = (text: string) =>
  [...document.body.querySelectorAll('button')].find((b) => b.textContent?.trim() === text) as HTMLButtonElement | undefined

async function mountList(txnId: string | null) {
  const w = mount(AttachmentList, { props: { txnId }, global: { plugins: [i18n] }, attachTo: document.body })
  await flushPromises()
  return w
}

describe('AttachmentList', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.spyOn(useTransactionsStore(), 'reload').mockResolvedValue()
  })
  afterEach(() => {
    vi.restoreAllMocks()
    document.body.innerHTML = ''
  })

  it('txnId null: hanya petunjuk simpan dulu', async () => {
    const spy = vi.spyOn(ipc, 'listAttachments')
    const w = await mountList(null)
    expect(w.text()).toContain(id.txn.attachments.saveFirst)
    expect(w.find('button').exists()).toBe(false)
    expect(spy).not.toHaveBeenCalled()
    w.unmount()
  })

  it('menampilkan nama dan ukuran lampiran', async () => {
    vi.spyOn(ipc, 'listAttachments').mockResolvedValue([png])
    const w = await mountList('t1')
    const row = w.get('[data-testid="attachment-row"]')
    expect(row.text()).toContain('struk.png')
    expect(row.text()).toContain('1,2 KB')
    w.unmount()
  })

  it('file hilang: status tampil dan tombol Buka nonaktif', async () => {
    vi.spyOn(ipc, 'listAttachments').mockResolvedValue([pdf])
    const w = await mountList('t1')
    const row = w.get('[data-testid="attachment-row"]')
    expect(row.text()).toContain(id.txn.attachments.missing)
    expect(row.text()).toContain('2,5 MB')
    expect(button(id.txn.attachments.open)!.disabled).toBe(true)
    w.unmount()
  })

  it('lampirkan file memanggil addAttachments; error tampil di alert', async () => {
    vi.spyOn(ipc, 'listAttachments').mockResolvedValue([])
    const add = vi.spyOn(ipc, 'addAttachments').mockRejectedValue(new IpcFailure({ code: 'attachment_too_large', entity: null }))
    const w = await mountList('t1')
    button(id.txn.attachments.add)!.click()
    await flushPromises()
    expect(add).toHaveBeenCalledWith('t1')
    expect(document.body.querySelector('[role="alert"]')?.textContent).toBe(id.errors.attachment_too_large)
    w.unmount()
  })

  it('tambah berhasil memuat ulang daftar dan transaksi', async () => {
    const list = vi.spyOn(ipc, 'listAttachments').mockResolvedValueOnce([]).mockResolvedValueOnce([png])
    vi.spyOn(ipc, 'addAttachments').mockResolvedValue([png])
    const w = await mountList('t1')
    button(id.txn.attachments.add)!.click()
    await flushPromises()
    expect(list).toHaveBeenCalledTimes(2)
    expect(w.text()).toContain('struk.png')
    expect(useTransactionsStore().reload).toHaveBeenCalled()
    w.unmount()
  })

  it('hapus meminta konfirmasi lalu memanggil removeAttachment', async () => {
    vi.spyOn(ipc, 'listAttachments').mockResolvedValue([png])
    const remove = vi.spyOn(ipc, 'removeAttachment').mockResolvedValue()
    const w = await mountList('t1')
    ;(w.get('[data-testid="attachment-remove"]').element as HTMLButtonElement).click()
    await flushPromises()
    expect(document.body.textContent).toContain('Hapus lampiran struk.png?')
    button(id.actions.delete)!.click()
    await flushPromises()
    expect(remove).toHaveBeenCalledWith('a1')
    w.unmount()
  })
})
