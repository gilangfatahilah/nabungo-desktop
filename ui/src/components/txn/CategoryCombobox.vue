<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { CategoryKindDto } from '@/ipc'
import { useCategoriesStore } from '@/stores/categories'
import Autocomplete, { type Option } from './Autocomplete.vue'

// Pilih kategori aktif sesuai jenis (tanpa `kind`: semua jenis, pengeluaran dulu);
// sub-kategori tampil sebagai `Parent › Child`.
const model = defineModel<string | null>({ required: true })
const props = defineProps<{ kind?: CategoryKindDto }>()
const emit = defineEmits<{ picked: [id: string | null] }>()
defineOptions({ inheritAttrs: false })

const store = useCategoriesStore()

const all = computed<Option[]>(() => {
  const byId = new Map(store.list.map((c) => [c.id, c]))
  return store.list
    .filter((c) => (!props.kind || c.kind === props.kind) && !c.archived)
    .map((c) => {
      const parent = c.parentId ? byId.get(c.parentId) : undefined
      const kindOrder = c.kind === 'expense' ? 0 : 1
      return { value: c.id, label: parent ? `${parent.name} › ${c.name}` : c.name, order: [kindOrder, parent?.sortOrder ?? c.sortOrder, parent ? c.sortOrder : 0] }
    })
    .sort((a, b) => a.order[0]! - b.order[0]! || a.order[1]! - b.order[1]! || a.order[2]! - b.order[2]!)
    .map(({ value, label }) => ({ value, label }))
})

const labelOf = (id: string | null) => all.value.find((o) => o.value === id)?.label
  ?? store.list.find((c) => c.id === id)?.name ?? ''

const text = ref(labelOf(model.value))
watch([model, all], () => {
  if (model.value !== null) text.value = labelOf(model.value)
})

const options = computed(() => {
  const q = text.value.trim().toLowerCase()
  if (!q || text.value === labelOf(model.value)) return all.value
  return all.value.filter((o) => o.label.toLowerCase().includes(q))
})

function onInput(v: string) {
  text.value = v
  if (model.value !== null && v !== labelOf(model.value)) {
    model.value = null
    emit('picked', null)
  }
}

function select(o: Option) {
  model.value = o.value
  text.value = o.label
  emit('picked', o.value)
}
</script>

<template>
  <Autocomplete v-bind="$attrs" :model-value="text" :options="options" @update:model-value="onInput" @select="select" />
</template>
