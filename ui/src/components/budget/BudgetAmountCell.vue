<script setup lang="ts">
import { nextTick, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Input } from '@/components/ui/input'
import { useMoneyFormat } from '@/composables/useMoneyFormat'
import { parseAmount } from '@/ipc'
import { errorMessage, type Translate } from '@/lib/errors'

// Sel budget: teks biasa, klik/Enter → input inline. Enter/blur/Tab menyimpan, Esc batal, kosong = hapus budget.
const props = defineProps<{ categoryId: string; name: string; amount: number | null }>()
const emit = defineEmits<{ save: [amount: number | null]; error: [message: string] }>()

const { t } = useI18n()
const editing = ref(false)
const text = ref('')
let closing = false

// Mode sembunyikan nominal hanya berlaku untuk teks; input edit tetap menampilkan nilai asli.
const { num: display } = useMoneyFormat()

function open() {
  text.value = props.amount === null ? '' : String(props.amount)
  editing.value = true
  closing = false
  void nextTick(() => {
    const el = document.querySelector<HTMLInputElement>(`[data-testid="budget-input-${props.categoryId}"]`)
    el?.focus()
    el?.select()
  })
}

// Nilai dibaca dari elemen: Input (useVModel pasif) baru meng-emit model setelah handler keydown berjalan.
async function commit(e: Event) {
  if (closing) return
  closing = true
  text.value = (e.target as HTMLInputElement).value
  try {
    const value = text.value.trim() === '' ? null : await parseAmount(text.value)
    if (value !== props.amount) emit('save', value)
  } catch (e) {
    emit('error', errorMessage(e, t as unknown as Translate))
  }
  editing.value = false
}

function cancel() {
  closing = true
  editing.value = false
}
</script>

<template>
  <Input
    v-if="editing" v-model="text" inputmode="decimal" autocomplete="off"
    :data-testid="`budget-input-${categoryId}`" :aria-label="t('budget.edit', { name })"
    class="num h-8 text-right"
    @keydown.enter.prevent="commit" @keydown.esc.prevent="cancel" @blur="commit"
  />
  <button
    v-else type="button" :data-testid="`budget-cell-${categoryId}`" :aria-label="t('budget.edit', { name })"
    class="num -mx-2 h-8 w-[calc(100%+1rem)] rounded-md px-2 text-right hover:bg-muted outline-none focus-visible:ring-3 focus-visible:ring-ring"
    @click="open"
  >
    <span v-if="amount === null" class="text-muted-foreground">—</span>
    <template v-else>{{ display(amount) }}</template>
  </button>
</template>
