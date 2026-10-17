import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { i18n } from '@/i18n'
import id from '@/i18n/id'
import type { CategoryDto } from '@/ipc'
import { useCategoriesStore } from '@/stores/categories'
import MergeCategoryDialog from './MergeCategoryDialog.vue'

function cat(cid: string, name: string, kind: CategoryDto['kind'] = 'expense', archived = false): CategoryDto {
  return { id: cid, parentId: null, kind, name, icon: null, color: null, sortOrder: 0, archived, inUse: true }
}

const makan = cat('makan', 'Makan')
const list = [makan, cat('jajan', 'Jajan'), cat('lama', 'Lama', 'expense', true), cat('gaji', 'Gaji', 'income')]

const key = (el: Element, k: string) => el.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true }))

describe('MergeCategoryDialog', () => {
  beforeEach(() => setActivePinia(createPinia()))
  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('hanya menawarkan target sejenis & aktif, lalu merge sekali', async () => {
    const store = useCategoriesStore()
    store.list = list
    const merge = vi.spyOn(store, 'mergeCategory').mockResolvedValue()
    const w = mount(MergeCategoryDialog, {
      props: { open: true, source: makan },
      global: { plugins: [i18n] },
      attachTo: document.body,
    })
    await flushPromises()

    key(document.body.querySelector('[data-testid="merge-target"]')!, 'Enter')
    await flushPromises()
    const options = [...document.body.querySelectorAll('[role="option"]')]
    expect(options.map((o) => o.textContent?.trim())).toEqual(['Jajan'])

    key(options[0]!, 'Enter')
    await flushPromises()
    expect(document.body.textContent).toContain('Semua transaksi dan budget Makan dipindah ke Jajan.')

    const confirm = [...document.body.querySelectorAll('button')].find((b) => b.textContent?.trim() === id.categories.merge)
    confirm!.click()
    confirm!.click()
    await flushPromises()
    expect(merge).toHaveBeenCalledTimes(1)
    expect(merge).toHaveBeenCalledWith('makan', 'jajan')
    w.unmount()
  })
})
