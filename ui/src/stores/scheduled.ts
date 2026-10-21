import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import * as ipc from '@/ipc'
import { IpcFailure, type RecurrenceDto, type RuleDto } from '@/ipc'
import { toDto, type TxnFormState } from '@/lib/txnForm'
import { useTransactionsStore } from './transactions'

// Transaksi terjadwal (TRX-06). Seperti store lain: setiap aksi tulis memuat ulang dari backend.
export const useScheduledStore = defineStore('scheduled', () => {
  const rules = ref<RuleDto[]>([])
  const loaded = ref(false)
  const dueCount = computed(() => rules.value.filter((r) => !r.ended).reduce((n, r) => n + r.due.length, 0))

  async function reload() {
    rules.value = await ipc.listRules()
    loaded.value = true
  }

  const input = (state: TxnFormState, recurrence: RecurrenceDto, autoPost: boolean) => ({ form: toDto(state), recurrence, autoPost })

  async function create(state: TxnFormState, recurrence: RecurrenceDto, autoPost: boolean): Promise<string> {
    const id = await ipc.createRule(input(state, recurrence, autoPost))
    await reload()
    return id
  }

  async function update(id: string, state: TxnFormState, recurrence: RecurrenceDto, autoPost: boolean) {
    await ipc.updateRule(id, input(state, recurrence, autoPost))
    await reload()
  }

  async function remove(id: string) {
    await ipc.deleteRule(id)
    await reload()
  }

  /** Kejadian sudah ditangani di tempat lain: muat ulang agar daftar tidak basi, lalu teruskan error. */
  async function guarded<T>(action: () => Promise<T>): Promise<T> {
    try {
      return await action()
    } catch (e) {
      if (e instanceof IpcFailure && e.error.code === 'occurrence_stale') await reload()
      throw e
    }
  }

  async function posted() {
    await Promise.all([reload(), useTransactionsStore().refresh()])
  }

  async function record(id: string, date: string, state: TxnFormState | null): Promise<string> {
    const saved = await guarded(() => ipc.confirmOccurrence(id, date, state && toDto(state)))
    await posted()
    await useTransactionsStore().notifyAlerts(saved.alerts)
    return saved.id
  }

  const confirm = (id: string, date: string) => record(id, date, null)
  const confirmEdited = (id: string, date: string, state: TxnFormState) => record(id, date, state)

  async function skip(id: string, date: string) {
    await guarded(() => ipc.skipOccurrence(id, date))
    await reload()
  }

  async function confirmAll(id: string): Promise<number> {
    const n = await ipc.confirmAllDue(id)
    await posted()
    return n
  }

  return { rules, loaded, dueCount, reload, create, update, remove, confirm, confirmEdited, skip, confirmAll }
})
