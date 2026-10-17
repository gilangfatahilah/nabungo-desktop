import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

export interface TxnEditor {
  mode: 'create' | 'edit' | 'duplicate'
  id?: string
}

// State overlay global: command palette dan sheet form transaksi.
export const useUiStore = defineStore('ui', () => {
  const paletteOpen = ref(false)
  const txnEditor = ref<TxnEditor | null>(null)
  const newTxnOpen = computed(() => txnEditor.value !== null)
  return {
    paletteOpen,
    txnEditor,
    newTxnOpen,
    openNewTxn: () => { txnEditor.value = { mode: 'create' } },
    openEditTxn: (id: string) => { txnEditor.value = { mode: 'edit', id } },
    openDuplicateTxn: (id: string) => { txnEditor.value = { mode: 'duplicate', id } },
    closeTxnEditor: () => { txnEditor.value = null },
  }
})
