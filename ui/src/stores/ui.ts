import { defineStore } from 'pinia'
import { ref } from 'vue'

// State overlay global: command palette dan sheet transaksi baru.
export const useUiStore = defineStore('ui', () => {
  const paletteOpen = ref(false)
  const newTxnOpen = ref(false)
  return { paletteOpen, newTxnOpen }
})
