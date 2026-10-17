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
})
