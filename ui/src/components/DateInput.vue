<script setup lang="ts">
import { type DateValue, parseDate } from '@internationalized/date'
import { CalendarIcon } from '@lucide/vue'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Calendar } from '@/components/ui/calendar'
import { Input } from '@/components/ui/input'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { formatDateInput, parseDateInput } from '@/lib/date'
import { useSettingsStore } from '@/stores/settings'

// Pengganti <input type="date">: tampil "17 Okt 2026" sesuai locale, nilai tetap `YYYY-MM-DD`.
// Teks bebas di-parse saat blur/Enter (docs/design/design.md §7.3).
const props = withDefaults(defineProps<{ modelValue: string | null; nullable?: boolean }>(), { nullable: false })
const emit = defineEmits<{ 'update:modelValue': [value: string | null] }>()
defineOptions({ inheritAttrs: false })

const { t } = useI18n()
const settings = useSettingsStore()
const display = (v: string | null) => (v ? formatDateInput(v, settings.locale) : '')
const text = ref(display(props.modelValue))
const invalid = ref(false)
const open = ref(false)
const inputEl = ref<InstanceType<typeof Input> | null>(null)

watch([() => props.modelValue, () => settings.locale], ([v]) => {
  text.value = display(v)
  invalid.value = false
})

/** Parse teks sekarang. Form wajib `commit()` sebelum membaca model: Ctrl+Enter bisa mendahului blur. */
function commit(): boolean {
  if (text.value.trim() === '' && props.nullable) {
    invalid.value = false
    if (props.modelValue !== null) emit('update:modelValue', null)
    return true
  }
  const parsed = parseDateInput(text.value, settings.locale)
  if (parsed === null) {
    invalid.value = true
    return false
  }
  invalid.value = false
  if (parsed !== props.modelValue) emit('update:modelValue', parsed)
  else text.value = display(parsed)
  return true
}

const calendarValue = computed(() => (props.modelValue ? parseDate(props.modelValue) : undefined))

function pick(v: DateValue | undefined) {
  if (!v) return
  emit('update:modelValue', v.toString())
  open.value = false
  ;(inputEl.value?.$el as HTMLInputElement | undefined)?.focus()
}

defineExpose({ commit })
</script>

<template>
  <Popover v-model:open="open">
    <div class="relative">
      <Input
        ref="inputEl"
        v-bind="$attrs"
        v-model="text"
        autocomplete="off"
        class="pr-8"
        :aria-invalid="invalid || undefined"
        @blur="commit"
        @keydown.enter="commit"
      />
      <PopoverTrigger
        type="button"
        class="absolute inset-y-0 right-0 flex w-8 items-center justify-center rounded-r-lg text-muted-foreground hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
        :aria-label="t('dateInput.openCalendar')"
      >
        <CalendarIcon class="size-4" />
      </PopoverTrigger>
    </div>
    <PopoverContent class="w-auto p-2" align="start">
      <Calendar
        :model-value="calendarValue"
        :locale="settings.locale === 'id' ? 'id-ID' : 'en-US'"
        :week-starts-on="1"
        initial-focus
        @update:model-value="pick"
      />
    </PopoverContent>
  </Popover>
</template>
