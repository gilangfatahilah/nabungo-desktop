import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as ipc from '@/ipc'
import type { PayeeUsageDto, TagUsageDto } from '@/ipc'
import { useTransactionsStore } from './transactions'

/** Tag & payee (CAT-04). Aksi tulis memuat ulang daftar ini dan daftar transaksi. */
export const useLabelsStore = defineStore('labels', () => {
  const tags = ref<TagUsageDto[]>([])
  const payees = ref<PayeeUsageDto[]>([])
  const loaded = ref(false)

  async function reload() {
    ;[tags.value, payees.value] = await Promise.all([ipc.listTagUsage(), ipc.listPayeeUsage()])
    loaded.value = true
  }

  async function then<T>(p: Promise<T>): Promise<T> {
    const r = await p
    await Promise.all([reload(), useTransactionsStore().reload()])
    return r
  }

  return {
    tags,
    payees,
    loaded,
    reload,
    createTag: (name: string) => then(ipc.createTag(name)),
    renameTag: (id: string, name: string) => then(ipc.renameTag(id, name)),
    mergeTag: (source: string, target: string) => then(ipc.mergeTag(source, target)),
    deleteTag: (id: string) => then(ipc.deleteTag(id)),
    createPayee: (name: string, defaultCategoryId: string | null) => then(ipc.createPayee(name, defaultCategoryId)),
    updatePayee: (id: string, name: string, defaultCategoryId: string | null) =>
      then(ipc.updatePayee(id, name, defaultCategoryId)),
    mergePayee: (source: string, target: string) => then(ipc.mergePayee(source, target)),
    deletePayee: (id: string) => then(ipc.deletePayee(id)),
  }
})
