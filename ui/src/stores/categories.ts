import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as ipc from '@/ipc'
import type { CategoryDto, CategoryFieldsDto, CategoryKindDto } from '@/ipc'

export const useCategoriesStore = defineStore('categories', () => {
  const list = ref<CategoryDto[]>([])
  const loaded = ref(false)

  async function load() {
    list.value = await ipc.listCategories()
    loaded.value = true
  }

  async function then<T>(p: Promise<T>): Promise<T> {
    const r = await p
    await load()
    return r
  }

  return {
    list,
    loaded,
    load,
    createCategory: (kind: CategoryKindDto, fields: CategoryFieldsDto) => then(ipc.createCategory(kind, fields)),
    updateCategory: (id: string, fields: CategoryFieldsDto) => then(ipc.updateCategory(id, fields)),
    archiveCategory: (id: string) => then(ipc.archiveCategory(id)),
    unarchiveCategory: (id: string) => then(ipc.unarchiveCategory(id)),
    deleteCategory: (id: string) => then(ipc.deleteCategory(id)),
    mergeCategory: (source: string, target: string) => then(ipc.mergeCategory(source, target)),
  }
})
