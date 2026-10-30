<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import IconSelect, { type IconOption } from '@/components/IconSelect.vue'
import type { CategoryDto, CategoryKindDto } from '@/ipc'
import { useCategoriesStore } from '@/stores/categories'

// Pemilih kategori dengan kotak cari + ikon. Tanpa `categories`: kategori aktif sesuai `kind`
// (tanpa `kind`: semua jenis, pengeluaran dulu). Sub-kategori tampil sebagai `Parent › Child`
// dan memakai ikon/warna induk bila tidak punya sendiri. Atribut jatuh ke tombol pemicu.
const model = defineModel<string | null>({ required: true })
const props = defineProps<{
  kind?: CategoryKindDto
  categories?: CategoryDto[]
  noneLabel?: string
  placeholder?: string
}>()
const emit = defineEmits<{ picked: [id: string | null] }>()
defineOptions({ inheritAttrs: false })

const { t } = useI18n()
const store = useCategoriesStore()

const options = computed<IconOption[]>(() => {
  const byId = new Map(store.list.map((c) => [c.id, c]))
  const list = props.categories
    ?? store.list.filter((c) => (!props.kind || c.kind === props.kind) && (!c.archived || c.id === model.value))
  return list
    .map((c) => {
      const parent = c.parentId ? byId.get(c.parentId) : undefined
      const kindOrder = c.kind === 'expense' ? 0 : 1
      return {
        option: {
          value: c.id,
          label: parent ? `${parent.name} › ${c.name}` : c.name,
          icon: c.icon ?? parent?.icon ?? null,
          color: c.color ?? parent?.color ?? null,
        },
        order: [kindOrder, parent?.sortOrder ?? c.sortOrder, parent ? c.sortOrder : 0],
      }
    })
    .sort((a, b) => a.order[0]! - b.order[0]! || a.order[1]! - b.order[1]! || a.order[2]! - b.order[2]!)
    .map((x) => x.option)
})
</script>

<template>
  <IconSelect
    v-bind="$attrs" v-model="model" :options="options" :none-label="noneLabel" :placeholder="placeholder"
    :search-placeholder="t('categories.search')" :empty-text="t('categories.searchEmpty')" @picked="emit('picked', $event)"
  />
</template>
