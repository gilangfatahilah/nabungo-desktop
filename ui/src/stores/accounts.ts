import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as ipc from '@/ipc'
import type { AccountFieldsDto, AccountKindDto, AccountsOverviewDto } from '@/ipc'

const EMPTY: AccountsOverviewDto = { groups: [], accounts: [], netWorth: { assets: 0, liabilities: 0, net: 0 } }

// Setiap aksi tulis memuat ulang overview: saldo & net worth selalu diturunkan dari backend.
export const useAccountsStore = defineStore('accounts', () => {
  const overview = ref<AccountsOverviewDto>(EMPTY)
  const loaded = ref(false)

  async function load() {
    overview.value = await ipc.accountsOverview()
    loaded.value = true
  }

  async function then<T>(p: Promise<T>): Promise<T> {
    const r = await p
    await load()
    return r
  }

  return {
    overview,
    loaded,
    load,
    createAccount: (kind: AccountKindDto, fields: AccountFieldsDto) => then(ipc.createAccount(kind, fields)),
    updateAccount: (id: string, fields: AccountFieldsDto) => then(ipc.updateAccount(id, fields)),
    archiveAccount: (id: string) => then(ipc.archiveAccount(id)),
    unarchiveAccount: (id: string) => then(ipc.unarchiveAccount(id)),
    deleteAccount: (id: string) => then(ipc.deleteAccount(id)),
    reorderAccounts: (groupId: string | null, ids: string[]) => then(ipc.reorderAccounts(groupId, ids)),
    createAccountGroup: (name: string) => then(ipc.createAccountGroup(name)),
    renameAccountGroup: (id: string, name: string) => then(ipc.renameAccountGroup(id, name)),
    deleteAccountGroup: (id: string) => then(ipc.deleteAccountGroup(id)),
    reorderAccountGroups: (ids: string[]) => then(ipc.reorderAccountGroups(ids)),
    recordValuation: (id: string, date: string, value: number) => then(ipc.recordValuation(id, date, value)),
    reconcile: (id: string, date: string, balance: number) => then(ipc.reconcileAccount(id, date, balance)),
  }
})
