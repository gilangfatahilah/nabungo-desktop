<script setup lang="ts">
import { XIcon } from '@lucide/vue'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { listTags } from '@/ipc'
import Autocomplete, { type Option } from './Autocomplete.vue'

// Tag: Enter atau koma menambah, Backspace di field kosong menghapus yang terakhir.
// Normalisasi final (trim, `#`, dedupe) tetap di Rust.
const tags = defineModel<string[]>({ required: true })
defineOptions({ inheritAttrs: false })

const { t } = useI18n()
const text = ref('')
const known = ref<string[]>([])

onMounted(async () => {
  known.value = await listTags()
})

const options = computed<Option[]>(() => {
  const q = text.value.trim().replace(/^#/, '').toLowerCase()
  if (!q) return []
  const used = new Set(tags.value.map((x) => x.toLowerCase()))
  return known.value
    .filter((x) => x.toLowerCase().includes(q) && !used.has(x.toLowerCase()))
    .slice(0, 8)
    .map((x) => ({ value: x, label: x }))
})

function add(raw: string) {
  const v = raw.trim().replace(/^#/, '').trim()
  text.value = ''
  if (v && !tags.value.some((x) => x.toLowerCase() === v.toLowerCase())) tags.value = [...tags.value, v]
}

function onInput(v: string) {
  if (v.includes(',')) {
    for (const part of v.split(',').slice(0, -1)) add(part)
    text.value = v.slice(v.lastIndexOf(',') + 1)
  } else {
    text.value = v
  }
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Backspace' && text.value === '' && tags.value.length) tags.value = tags.value.slice(0, -1)
}
</script>

<template>
  <div class="flex flex-col gap-2">
    <ul v-if="tags.length" class="flex flex-wrap gap-1" data-testid="tags">
      <li v-for="tag in tags" :key="tag" class="flex items-center gap-1 rounded-full bg-secondary px-2 py-0.5 text-xs">
        #{{ tag }}
        <button type="button" :aria-label="t('txn.removeTag', { tag })" @click="tags = tags.filter((x) => x !== tag)">
          <XIcon class="size-3" />
        </button>
      </li>
    </ul>
    <Autocomplete
      v-bind="$attrs"
      :model-value="text"
      :options="options"
      @update:model-value="onInput"
      @select="add($event.value)"
      @enter="add"
      @keydown="onKeydown"
      @blur="add(text)"
    />
  </div>
</template>
