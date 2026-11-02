<script setup lang="ts">
import { type DateValue, parseDate } from '@internationalized/date'
import { CalendarIcon } from '@lucide/vue'
import type { DateRange } from 'reka-ui'
import { shallowRef, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { RangeCalendar } from '@/components/ui/range-calendar'
import { formatDateInput } from '@/lib/date'
import { cn } from '@/lib/utils'
import { useSettingsStore } from '@/stores/settings'

// Date range picker (Popover + RangeCalendar). Nilai `YYYY-MM-DD` inklusif; emit hanya
// setelah kedua ujung dipilih. Atribut (aria-*, data-testid, class) jatuh ke tombol pemicu.
const props = defineProps<{ from: string | null; to: string | null; placeholder?: string }>()
const emit = defineEmits<{ change: [range: { from: string; to: string }] }>()
const open = defineModel<boolean>('open', { default: false })
defineOptions({ inheritAttrs: false })

const settings = useSettingsStore()

const toRange = (): DateRange => ({
  start: props.from ? parseDate(props.from) : undefined,
  end: props.to ? parseDate(props.to) : undefined,
})
// Draft terpisah dari props: pilihan setengah jadi tidak mengubah filter.
const draft = shallowRef<DateRange>(toRange())
watch([() => props.from, () => props.to, open], () => { draft.value = toRange() })

function pick(r: DateRange) {
  draft.value = r
  if (!r.start || !r.end) return
  emit('change', { from: r.start.toString(), to: r.end.toString() })
  open.value = false
}

const fmt = (v: string) => formatDateInput(v, settings.locale)
const placeholderDate = () => (props.from ? parseDate(props.from) : undefined) as DateValue | undefined
</script>

<template>
  <Popover v-model:open="open">
    <PopoverTrigger as-child>
      <Button
        variant="outline" v-bind="$attrs"
        :class="cn('justify-start font-normal', !from && 'text-muted-foreground', $attrs.class as string)"
      >
        <CalendarIcon class="text-muted-foreground" />
        <span v-if="from && to" class="num truncate">{{ fmt(from) }} – {{ fmt(to) }}</span>
        <span v-else class="truncate">{{ placeholder }}</span>
      </Button>
    </PopoverTrigger>
    <PopoverContent class="w-auto p-0" align="start">
      <RangeCalendar
        :model-value="draft"
        :default-placeholder="placeholderDate()"
        :locale="settings.locale === 'id' ? 'id-ID' : 'en-US'"
        :week-starts-on="1"
        :number-of-months="2"
        initial-focus
        @update:model-value="pick"
      />
    </PopoverContent>
  </Popover>
</template>
