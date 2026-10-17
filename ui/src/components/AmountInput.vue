<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Input } from '@/components/ui/input'
import { parseAmount } from '@/ipc'
import { errorMessage, type Translate } from '@/lib/errors'

// Teks bebas (mis. `1,5jt`, `25000+12500`) di-parse oleh Rust saat blur/Enter.
const model = defineModel<number>({ required: true })
const emit = defineEmits<{ error: [message: string | null] }>()
defineOptions({ inheritAttrs: false })

const { t } = useI18n()
const text = ref(String(model.value))
const invalid = ref(false)

watch(model, (v) => {
  if (!invalid.value) text.value = String(v)
})

/** Parse teks sekarang. Form wajib `await commit()` sebelum membaca model: Enter/klik submit bisa mendahului blur. */
async function commit(): Promise<boolean> {
  try {
    model.value = await parseAmount(text.value)
    text.value = String(model.value)
    invalid.value = false
    emit('error', null)
    return true
  } catch (e) {
    invalid.value = true
    emit('error', errorMessage(e, t as unknown as Translate))
    return false
  }
}

defineExpose({ commit })
</script>

<template>
  <Input
    v-bind="$attrs"
    v-model="text"
    inputmode="decimal"
    autocomplete="off"
    :aria-invalid="invalid || undefined"
    @blur="commit"
    @keydown.enter="commit"
  />
</template>
