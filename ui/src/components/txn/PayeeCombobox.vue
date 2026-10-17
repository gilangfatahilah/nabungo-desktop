<script setup lang="ts">
import { ref, watch } from 'vue'
import { searchPayees } from '@/ipc'
import Autocomplete, { type Option } from './Autocomplete.vue'

// Payee teks bebas; saran dari payee yang pernah dipakai. `picked` dipancarkan saat
// saran dipilih atau field ditinggalkan, agar form bisa meminta saran kategori/akun.
const model = defineModel<string>({ required: true })
const emit = defineEmits<{ picked: [name: string] }>()
defineOptions({ inheritAttrs: false })

const options = ref<Option[]>([])
let seq = 0

watch(model, async (q) => {
  const n = ++seq
  const names = q.trim() ? await searchPayees(q) : []
  if (n === seq) options.value = names.filter((x) => x !== q).map((x) => ({ value: x, label: x }))
})

function select(o: Option) {
  model.value = o.value
  emit('picked', o.value)
}
</script>

<template>
  <Autocomplete
    v-bind="$attrs"
    v-model="model"
    :options="options"
    @select="select"
    @enter="emit('picked', $event)"
    @change="emit('picked', model)"
  />
</template>
