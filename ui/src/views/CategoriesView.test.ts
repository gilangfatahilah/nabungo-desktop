import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import { IpcFailure, type CategoryDto } from '@/ipc'
import { useCategoriesStore } from '@/stores/categories'
import CategoriesView from './CategoriesView.vue'

const makan: CategoryDto = {
  id: 'makan', parentId: null, kind: 'expense', name: 'Makan', icon: null, color: null, sortOrder: 0, archived: false, inUse: true,
}

const key = (el: Element, k: string) => el.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true }))
const button = (text: string) =>
  [...document.body.querySelectorAll('button, [role="menuitem"]')].find((b) => b.textContent?.trim() === text) as HTMLElement

describe('CategoriesView', () => {
  beforeEach(() => setActivePinia(createPinia()))
  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('hapus kategori terpakai menawarkan arsip atau gabung', async () => {
    const store = useCategoriesStore()
    store.list = [makan]
    store.loaded = true
    vi.spyOn(store, 'load').mockResolvedValue()
    const del = vi.spyOn(store, 'deleteCategory').mockRejectedValue(new IpcFailure({ code: 'in_use', entity: 'category' }))
    const archive = vi.spyOn(store, 'archiveCategory').mockResolvedValue()
    const w = mount(CategoriesView, { global: { plugins: [i18n] }, attachTo: document.body })
    await flushPromises()

    key(document.body.querySelector('[data-testid="category-menu-makan"]')!, 'Enter')
    await flushPromises()
    button(id.actions.delete).click()
    await flushPromises()
    button(id.actions.delete).click()
    await flushPromises()
    expect(del).toHaveBeenCalledWith('makan')
    expect(document.body.textContent).toContain(id.categories.inUseBody)
    expect(button(id.categories.merge)).toBeTruthy()

    button(id.actions.archive).click()
    await flushPromises()
    expect(archive).toHaveBeenCalledWith('makan')
    w.unmount()
  })

  it('segmented control mengganti jenis; anak menjorok; arsip terlipat', async () => {
    const store = useCategoriesStore()
    store.list = [
      makan,
      { ...makan, id: 'resto', parentId: 'makan', name: 'Restoran' },
      { ...makan, id: 'gaji', kind: 'income', name: 'Gaji' },
      { ...makan, id: 'lama', name: 'Rokok', archived: true },
    ]
    store.loaded = true
    vi.spyOn(store, 'load').mockResolvedValue()
    const w = mount(CategoriesView, { global: { plugins: [i18n] }, attachTo: document.body })
    await flushPromises()

    const rows = w.findAll('[data-testid="category-row"]')
    expect(rows.map((r) => [r.get('[data-part="name"]').text(), r.attributes('data-depth')])).toEqual([['Makan', '0'], ['Restoran', '1']])
    expect(w.text()).not.toContain('Gaji')

    const income = w.findAll('[role="tab"]').find((t) => t.text() === id.categories.income)!
    await income.trigger('mousedown', { button: 0 })
    await flushPromises()
    expect(w.findAll('[data-testid="category-row"]').map((r) => r.get('[data-part="name"]').text())).toEqual(['Gaji'])

    const toggle = w.get('[data-testid="archived-toggle"]')
    expect(toggle.text()).toContain(`${id.categories.archived} (1)`)
    expect(w.text()).not.toContain('Rokok')
    await toggle.trigger('click')
    expect(w.text()).toContain('Rokok')
    w.unmount()
  })
})
