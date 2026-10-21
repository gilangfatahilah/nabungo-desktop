<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import DateInput from '@/components/DateInput.vue'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { type FreqDto, previewRecurrence, type RecurrenceDto } from '@/ipc'
import { formatDateInput } from '@/lib/date'
import { useSettingsStore } from '@/stores/settings'

// Field pengulangan di RuleSheet (K14). Validasi tetap di Rust saat simpan; pratinjau yang gagal disembunyikan.
const rec = defineModel<RecurrenceDto>({ required: true })
const autoPost = defineModel<boolean>('autoPost', { required: true })
const props = defineProps<{ start: string }>()

const { t } = useI18n()
const settings = useSettingsStore()
const FREQS: FreqDto[] = ['daily', 'weekly', 'monthly', 'yearly']

const ends = computed({
  get: () => (rec.value.until === null ? 'never' : 'on'),
  set: (v) => { rec.value = { ...rec.value, until: v === 'never' ? null : props.start } },
})

const preview = ref<string[]>([])
let timer: ReturnType<typeof setTimeout> | undefined
watch([rec, () => props.start], () => {
  clearTimeout(timer)
  timer = setTimeout(async () => {
    try {
      preview.value = await previewRecurrence(rec.value, props.start, 3)
    } catch {
      preview.value = []
    }
  }, 200)
}, { immediate: true, deep: true })
onUnmounted(() => clearTimeout(timer))
</script>

<template>
  <div data-testid="recurrence-fields" class="flex flex-col gap-4 border-t pt-4">
    <div class="grid grid-cols-2 gap-3">
      <div class="flex flex-col gap-2">
        <Label id="rule-freq-label">{{ t('scheduled.frequency') }}</Label>
        <Select :model-value="rec.freq" @update:model-value="rec = { ...rec, freq: $event as FreqDto }">
          <SelectTrigger aria-labelledby="rule-freq-label" class="w-full" data-testid="rule-freq"><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem v-for="f in FREQS" :key="f" :value="f">{{ t(`scheduled.freq.${f}`) }}</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div class="flex flex-col gap-2">
        <Label for="rule-interval">{{ t('scheduled.interval') }}</Label>
        <div class="flex items-center gap-2">
          <Input
            id="rule-interval" type="number" min="1" max="99" class="num w-20 text-right" data-testid="rule-interval"
            :model-value="rec.interval" @update:model-value="rec = { ...rec, interval: Number($event) }"
          />
          <span class="text-muted-foreground">{{ t(`scheduled.unit.${rec.freq}`) }}</span>
        </div>
      </div>
    </div>
    <div class="grid grid-cols-2 gap-3">
      <div class="flex flex-col gap-2">
        <Label id="rule-ends-label">{{ t('scheduled.ends') }}</Label>
        <Select v-model="ends">
          <SelectTrigger aria-labelledby="rule-ends-label" class="w-full" data-testid="rule-ends"><SelectValue /></SelectTrigger>
          <SelectContent>
            <SelectItem value="never">{{ t('scheduled.endsNever') }}</SelectItem>
            <SelectItem value="on">{{ t('scheduled.endsOn') }}</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div v-if="rec.until !== null" class="flex flex-col gap-2">
        <Label for="rule-until">{{ t('scheduled.until') }}</Label>
        <DateInput id="rule-until" :model-value="rec.until" @update:model-value="rec = { ...rec, until: $event }" />
      </div>
    </div>
    <Label class="flex items-start gap-3 font-normal">
      <Switch v-model="autoPost" data-testid="rule-auto-post" class="mt-0.5" />
      <span class="flex flex-col gap-0.5">
        <span class="font-medium">{{ t('scheduled.autoPost') }}</span>
        <span class="text-meta text-muted-foreground">{{ t('scheduled.autoPostHint') }}</span>
      </span>
    </Label>
    <div v-if="preview.length" class="flex flex-col gap-1">
      <span class="text-meta text-muted-foreground">{{ t('scheduled.upcoming') }}</span>
      <ul data-testid="recurrence-preview" class="num flex flex-wrap gap-x-3 gap-y-1 text-meta">
        <li v-for="d in preview" :key="d">{{ formatDateInput(d, settings.locale) }}</li>
      </ul>
    </div>
  </div>
</template>
