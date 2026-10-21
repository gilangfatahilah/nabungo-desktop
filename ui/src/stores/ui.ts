import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

export interface TxnEditor {
  mode: 'create' | 'edit' | 'duplicate'
  id?: string
}

/** Sheet jadwal berulang (TRX-06, K14). */
export type RuleEditor =
  | { mode: 'create' }
  | { mode: 'edit'; id: string }
  | { mode: 'fromTxn'; txnId: string }
  | { mode: 'occurrence'; ruleId: string; date: string }

// State overlay global: command palette dan sheet form transaksi.
export const useUiStore = defineStore('ui', () => {
  const paletteOpen = ref(false)
  const txnEditor = ref<TxnEditor | null>(null)
  const newTxnOpen = computed(() => txnEditor.value !== null)
  const ruleEditor = ref<RuleEditor | null>(null)
  return {
    paletteOpen,
    txnEditor,
    newTxnOpen,
    openNewTxn: () => { txnEditor.value = { mode: 'create' } },
    openEditTxn: (id: string) => { txnEditor.value = { mode: 'edit', id } },
    openDuplicateTxn: (id: string) => { txnEditor.value = { mode: 'duplicate', id } },
    closeTxnEditor: () => { txnEditor.value = null },
    ruleEditor,
    openRuleEditor: (e: RuleEditor) => { ruleEditor.value = e },
    closeRuleEditor: () => { ruleEditor.value = null },
  }
})
