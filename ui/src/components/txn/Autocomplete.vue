<script setup lang="ts">
import { computed, ref, useId, watch } from 'vue'
import { Input } from '@/components/ui/input'

// Combobox ringan: input teks bebas + listbox di bawahnya (tidak di-teleport, aman di dalam sheet).
// Enter memilih opsi aktif; Ctrl/Cmd+Enter dibiarkan naik ke form untuk menyimpan.
export interface Option {
  value: string
  label: string
}

const text = defineModel<string>({ required: true })
const props = defineProps<{ options: Option[] }>()
const emit = defineEmits<{ select: [option: Option]; enter: [text: string] }>()
defineOptions({ inheritAttrs: false })

const listId = useId()
const open = ref(false)
const active = ref(-1)

watch([text, () => props.options], () => {
  active.value = text.value.trim() && props.options.length ? 0 : -1
})

const visible = computed(() => open.value && props.options.length > 0)

function choose(o: Option) {
  open.value = false
  emit('select', o)
}

function onKeydown(e: KeyboardEvent) {
  if (e.ctrlKey || e.metaKey) return
  const n = props.options.length
  if (e.key === 'ArrowDown' && n) {
    e.preventDefault()
    open.value = true
    active.value = (active.value + 1) % n
  } else if (e.key === 'ArrowUp' && n) {
    e.preventDefault()
    open.value = true
    active.value = (active.value - 1 + n) % n
  } else if (e.key === 'Enter') {
    e.preventDefault()
    const o = visible.value ? props.options[active.value] : undefined
    if (o) choose(o)
    else emit('enter', text.value)
  } else if (e.key === 'Escape' && visible.value) {
    // Tutup daftar saja, jangan tutup sheet.
    e.stopPropagation()
    open.value = false
  }
}
</script>

<template>
  <div class="relative">
    <Input
      v-bind="$attrs"
      v-model="text"
      role="combobox"
      autocomplete="off"
      :aria-expanded="visible"
      :aria-controls="listId"
      :aria-activedescendant="visible && active >= 0 ? `${listId}-${active}` : undefined"
      @input="open = true"
      @focus="open = true"
      @blur="open = false"
      @keydown="onKeydown"
    />
    <ul
      v-show="visible"
      :id="listId"
      role="listbox"
      class="absolute inset-x-0 top-full z-50 mt-1 max-h-60 overflow-auto rounded-md border bg-popover p-1 text-sm shadow-md"
    >
      <li
        v-for="(o, i) in options"
        :id="`${listId}-${i}`"
        :key="o.value"
        role="option"
        :aria-selected="i === active"
        class="cursor-pointer rounded-sm px-2 py-1.5 aria-selected:bg-accent aria-selected:text-accent-foreground"
        @mousedown.prevent="choose(o)"
        @mouseenter="active = i"
      >
        {{ o.label }}
      </li>
    </ul>
  </div>
</template>
