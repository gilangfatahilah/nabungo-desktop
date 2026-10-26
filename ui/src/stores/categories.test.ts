import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import * as ipc from '@/ipc'
import { useCategoriesStore } from './categories'

describe('categories store (fallback browser)', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('create lalu archive memperbarui list', async () => {
    const s = useCategoriesStore()
    const id = await s.createCategory('expense', { name: 'Hewan', parentId: null, icon: null, color: null })
    expect(s.list.find((c) => c.id === id)?.archived).toBe(false)
    await s.archiveCategory(id)
    expect(s.list.find((c) => c.id === id)?.archived).toBe(true)
  })
})

describe('categories ensureLoaded', () => {
  beforeEach(() => setActivePinia(createPinia()))
  afterEach(() => vi.restoreAllMocks())

  it('pemanggil bersamaan berbagi satu request, lalu tidak memuat lagi', async () => {
    const spy = vi.spyOn(ipc, 'listCategories')
    const s = useCategoriesStore()
    await Promise.all([s.ensureLoaded(), s.ensureLoaded()])
    expect(spy).toHaveBeenCalledTimes(1)
    expect(s.loaded).toBe(true)
    await s.ensureLoaded()
    expect(spy).toHaveBeenCalledTimes(1)
  })

  it('gagal → panggilan berikutnya mencoba lagi', async () => {
    const spy = vi.spyOn(ipc, 'listCategories').mockRejectedValueOnce(new Error('x'))
    const s = useCategoriesStore()
    await expect(s.ensureLoaded()).rejects.toThrow('x')
    await s.ensureLoaded()
    expect(spy).toHaveBeenCalledTimes(2)
    expect(s.loaded).toBe(true)
  })
})
