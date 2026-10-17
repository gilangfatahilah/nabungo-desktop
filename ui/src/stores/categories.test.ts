import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it } from 'vitest'
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
