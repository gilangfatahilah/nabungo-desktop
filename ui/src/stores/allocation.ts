import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as ipc from '@/ipc'
import type { AllocationOverviewDto, AllocationPointDto, BucketFieldsDto, ShareDto } from '@/ipc'
import { todayIso } from '@/lib/date'

// Jumlah periode pada riwayat porsi.
const HISTORY = 12

// Porsi aktual & selisih selalu dihitung backend; store hanya memuat ulang setelah aksi tulis.
export const useAllocationStore = defineStore('allocation', () => {
  const overview = ref<AllocationOverviewDto | null>(null)
  const history = ref<AllocationPointDto[]>([])

  async function load() {
    const [o, h] = await Promise.all([
      ipc.allocationOverview(),
      ipc.periodOf(todayIso()).then((p) => ipc.allocationHistory(p, HISTORY)),
    ])
    overview.value = o
    history.value = h
  }

  async function then<T>(p: Promise<T>): Promise<T> {
    const r = await p
    await load()
    return r
  }

  return {
    overview,
    history,
    load,
    createBucket: (fields: BucketFieldsDto) => then(ipc.createBucket(fields)),
    updateBucket: (id: string, fields: BucketFieldsDto) => then(ipc.updateBucket(id, fields)),
    deleteBucket: (id: string) => then(ipc.deleteBucket(id)),
    setAccountAllocation: (accountId: string, shares: ShareDto[]) => then(ipc.setAccountAllocation(accountId, shares)),
  }
})
