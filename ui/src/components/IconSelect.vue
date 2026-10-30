<script setup lang="ts">
import { ChevronDownIcon } from '@lucide/vue'
import { computed, ref } from 'vue'
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from '@/components/ui/command'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { colorClass } from '@/lib/colors'
import { iconFor } from '@/lib/icons'
import { cn } from '@/lib/utils'

// Select dengan kotak cari + ikon berwarna (dasar AccountSelect/CategorySelect).
// Atribut (id, aria-*, data-testid, class, disabled) jatuh ke tombol pemicu.
// `noneLabel` menambah opsi "kosong" (nilai null) di atas daftar; `placeholder` tampil saat null.
export interface IconOption {
  value: string
  label: string
  icon: string | null
  color: string | null
  disabled?: boolean
}

const model = defineModel<string | null>({ required: true })
const props = defineProps<{
  options: IconOption[]
  searchPlaceholder: string
  emptyText: string
  noneLabel?: string
  placeholder?: string
}>()
const emit = defineEmits<{ picked: [value: string | null] }>()
defineOptions({ inheritAttrs: false })

// Listbox tidak menerima null; sentinel untuk opsi kosong.
const NONE = '__none'
const open = ref(false)
const current = computed(() => props.options.find((o) => o.value === model.value))

function pick(value: string | null) {
  model.value = value
  open.value = false
  emit('picked', value)
}
</script>

<template>
  <Popover v-model:open="open">
    <PopoverTrigger as-child>
      <button
        type="button"
        role="combobox"
        :aria-expanded="open"
        v-bind="$attrs"
        :class="cn(
          'border-input dark:bg-input-fill/30 dark:hover:bg-input-fill/50 focus-visible:border-ring focus-visible:ring-ring aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive dark:aria-invalid:border-destructive/50 flex h-8 w-fit items-center justify-between gap-1.5 rounded-lg border bg-transparent py-2 pr-2 pl-2.5 text-sm whitespace-nowrap transition-colors outline-none select-none focus-visible:ring-3 aria-invalid:ring-3 disabled:cursor-not-allowed disabled:opacity-50',
          $attrs.class as string,
        )"
      >
        <span class="flex min-w-0 items-center gap-1.5">
          <template v-if="current">
            <span :class="cn('flex size-5 shrink-0 items-center justify-center rounded', colorClass(current.color))">
              <component :is="iconFor(current.icon)" class="size-3.5" />
            </span>
            <span class="truncate">{{ current.label }}</span>
          </template>
          <span v-else class="truncate text-muted-foreground">{{ placeholder ?? noneLabel }}</span>
        </span>
        <ChevronDownIcon class="pointer-events-none size-4 shrink-0 text-muted-foreground" />
      </button>
    </PopoverTrigger>
    <PopoverContent align="start" class="w-(--reka-popover-trigger-width) min-w-56 p-0">
      <Command :model-value="model ?? NONE">
        <CommandInput :placeholder="searchPlaceholder" />
        <CommandList>
          <CommandEmpty>{{ emptyText }}</CommandEmpty>
          <!-- CommandItem wajib di dalam CommandGroup (context pencarian). -->
          <CommandGroup>
            <CommandItem
              v-if="noneLabel" :value="NONE" class="text-muted-foreground [&[data-state=checked]>svg:last-child]:opacity-100"
              @select="pick(null)"
            >
              {{ noneLabel }}
            </CommandItem>
            <CommandItem
              v-for="o in options" :key="o.value" :value="o.value" :disabled="o.disabled"
              class="[&[data-state=checked]>svg:last-child]:opacity-100" @select="pick(o.value)"
            >
              <span :class="cn('flex size-5 shrink-0 items-center justify-center rounded', colorClass(o.color))">
                <component :is="iconFor(o.icon)" class="size-3.5" />
              </span>
              <span class="truncate">{{ o.label }}</span>
            </CommandItem>
          </CommandGroup>
        </CommandList>
      </Command>
    </PopoverContent>
  </Popover>
</template>
