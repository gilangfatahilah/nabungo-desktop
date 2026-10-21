import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createMemoryHistory, createRouter } from 'vue-router'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import { IpcFailure, type CategoryDto } from '@/ipc'
import { useCategoriesStore } from '@/stores/categories'
import { useLabelsStore } from '@/stores/labels'
import { useTransactionsStore } from '@/stores/transactions'
import LabelsView from './LabelsView.vue'

const makan: CategoryDto = {
  id: 'makan', parentId: null, kind: 'expense', name: 'Makan', icon: null, color: null, sortOrder: 0, archived: false, inUse: true,
}

const key = (el: Element, k: string) => el.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true }))
const button = (text: string) =>
  [...document.body.querySelectorAll('button, [role="menuitem"]')].find((b) => b.textContent?.trim() === text) as HTMLElement

function setup(query: Record<string, string> = {}) {
  const labels = useLabelsStore()
  labels.tags = [{ id: 'kantor', name: 'kantor', txnCount: 2 }, { id: 'rumah', name: 'rumah', txnCount: 0 }]
  labels.payees = [{ id: 'indomaret', name: 'Indomaret', defaultCategoryId: 'makan', txnCount: 3, lastDate: '2026-10-02' }]
  labels.loaded = true
  vi.spyOn(labels, 'reload').mockResolvedValue()
  const cats = useCategoriesStore()
  cats.list = [makan]
  cats.loaded = true
  vi.spyOn(cats, 'load').mockResolvedValue()
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/labels', name: 'labels', component: LabelsView },
      { path: '/transactions', name: 'transactions', component: { template: '<div />' } },
    ],
  })
  return { labels, router, mountView: async () => {
    await router.push({ path: '/labels', query })
    await router.isReady()
    const w = mount(LabelsView, { global: { plugins: [i18n, router] }, attachTo: document.body })
    await flushPromises()
    return w
  } }
}

async function openMenu(rowId: string, item: string) {
  key(document.body.querySelector(`[data-testid="label-menu-${rowId}"]`)!, 'Enter')
  await flushPromises()
  button(item).click()
  await flushPromises()
}

describe('LabelsView', () => {
  beforeEach(() => setActivePinia(createPinia()))
  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('memuat ulang tag/payee setiap kali dibuka (jumlah transaksi bisa berubah di layar lain)', async () => {
    const { labels, mountView } = setup()
    await mountView()
    expect(labels.reload).toHaveBeenCalledTimes(1)
  })

  it('tab Tag menampilkan #kantor dengan jumlah transaksi', async () => {
    const { mountView } = setup()
    const w = await mountView()
    const row = w.findAll('[data-testid="label-row"]')[0]!
    expect(row.text()).toContain('#kantor')
    expect(row.text()).toContain('2 transaksi')
    w.unmount()
  })

  it('tab Payee menampilkan payee beserta kategori default', async () => {
    const { mountView } = setup({ tab: 'payee' })
    const w = await mountView()
    const row = w.get('[data-testid="label-row"]')
    expect(row.text()).toContain('Indomaret')
    expect(row.text()).toContain('3 transaksi')
    expect(row.text()).toContain('Makan')
    w.unmount()
  })

  it('lihat transaksi tag mereset filter lain lalu pindah ke /transactions', async () => {
    const { router, mountView } = setup()
    const txns = useTransactionsStore()
    const setQuery = vi.spyOn(txns, 'setQuery').mockResolvedValue()
    const w = await mountView()
    await openMenu('kantor', id.labels.viewTxns)
    await flushPromises()
    expect(setQuery).toHaveBeenCalledWith(expect.objectContaining({ tag: 'kantor', payeeId: null, categoryId: null, search: null }))
    expect(router.currentRoute.value.path).toBe('/transactions')
    w.unmount()
  })

  it('hapus meminta konfirmasi dengan jumlah transaksi', async () => {
    const { labels, mountView } = setup()
    const del = vi.spyOn(labels, 'deleteTag').mockResolvedValue()
    const w = await mountView()
    await openMenu('kantor', id.actions.delete)
    expect(document.body.textContent).toContain('#kantor dilepas dari 2 transaksi. Saldo tidak berubah.')
    button(id.actions.delete).click()
    await flushPromises()
    expect(del).toHaveBeenCalledWith('kantor')
    w.unmount()
  })

  it('gabungkan memilih target lalu memanggil mergeTag', async () => {
    const { labels, mountView } = setup()
    const merge = vi.spyOn(labels, 'mergeTag').mockResolvedValue()
    const w = await mountView()
    await openMenu('kantor', `${id.labels.merge}…`)
    key(document.body.querySelector('[data-testid="merge-target"]')!, 'Enter')
    await flushPromises()
    const options = [...document.body.querySelectorAll('[role="option"]')]
    expect(options.map((o) => o.textContent?.trim())).toEqual(['#rumah'])
    key(options[0]!, 'Enter')
    await flushPromises()
    expect(document.body.textContent).toContain('Semua transaksi #kantor dipindah ke #rumah.')
    button(id.labels.merge).click()
    await flushPromises()
    expect(merge).toHaveBeenCalledWith('kantor', 'rumah')
    w.unmount()
  })

  it('ubah payee mengirim nama dan kategori default', async () => {
    const { labels, mountView } = setup({ tab: 'payee' })
    const update = vi.spyOn(labels, 'updatePayee').mockResolvedValue()
    const w = await mountView()
    await openMenu('indomaret', id.actions.edit)
    const name = document.body.querySelector<HTMLInputElement>('#label-name')!
    expect(name.value).toBe('Indomaret')
    name.value = 'Indomaret Point'
    name.dispatchEvent(new Event('input', { bubbles: true }))
    await flushPromises()
    document.body.querySelector('[data-testid="label-form"]')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await flushPromises()
    expect(update).toHaveBeenCalledWith('indomaret', 'Indomaret Point', 'makan')
    w.unmount()
  })

  it('error duplicate tampil di dialog', async () => {
    const { labels, mountView } = setup()
    vi.spyOn(labels, 'createTag').mockRejectedValue(new IpcFailure({ code: 'duplicate', entity: 'tag' }))
    const w = await mountView()
    button(`${id.labels.newTag}`).click()
    await flushPromises()
    const name = document.body.querySelector<HTMLInputElement>('#label-name')!
    name.value = 'kantor'
    name.dispatchEvent(new Event('input', { bubbles: true }))
    await flushPromises()
    document.body.querySelector('[data-testid="label-form"]')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }))
    await flushPromises()
    expect(document.body.querySelector('[role="alert"]')?.textContent).toContain('sudah ada')
    w.unmount()
  })
})
