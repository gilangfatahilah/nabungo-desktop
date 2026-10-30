import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { TxnFormKindDto } from '@/ipc'
import { loadAmountsHidden, saveAmountsHidden } from '@/lib/privacy'

export interface TxnEditor {
  mode: 'create' | 'edit' | 'duplicate'
  id?: string
  /** Mode create: jenis & akun awal (aksi cepat kartu akun di Dasbor). */
  preset?: { kind: TxnFormKindDto; accountId: string }
}

/** Sheet jadwal berulang (TRX-06, K14). */
export type RuleEditor =
  | { mode: 'create' }
  | { mode: 'edit'; id: string }
  | { mode: 'fromTxn'; txnId: string }
  | { mode: 'occurrence'; ruleId: string; date: string }

// State overlay global: command palette, sheet form transaksi, dan mode sembunyikan nominal.
export const useUiStore = defineStore('ui', () => {
  const paletteOpen = ref(false)
  const txnEditor = ref<TxnEditor | null>(null)
  const newTxnOpen = computed(() => txnEditor.value !== null)
  const ruleEditor = ref<RuleEditor | null>(null)
  const amountsHidden = ref(loadAmountsHidden())
  return {
    paletteOpen,
    txnEditor,
    newTxnOpen,
    openNewTxn: (preset?: TxnEditor['preset']) => { txnEditor.value = { mode: 'create', preset } },
    openEditTxn: (id: string) => { txnEditor.value = { mode: 'edit', id } },
    openDuplicateTxn: (id: string) => { txnEditor.value = { mode: 'duplicate', id } },
    closeTxnEditor: () => { txnEditor.value = null },
    ruleEditor,
    openRuleEditor: (e: RuleEditor) => { ruleEditor.value = e },
    closeRuleEditor: () => { ruleEditor.value = null },
    amountsHidden,
    toggleAmountsHidden: () => {
      amountsHidden.value = !amountsHidden.value
      saveAmountsHidden(amountsHidden.value)
    },
  }
})
