<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import DateInput from '@/components/DateInput.vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import type { CaptureDto } from '@/ipc'
import { formatDateInput, todayIso } from '@/lib/date'
import { useSettingsStore } from '@/stores/settings'

// Abaikan massal backlog lama (mis. sync pertama). Pratinjau dihitung dari daftar yang sedang tampil.
const props = defineProps<{ captures: CaptureDto[] }>()
const emit = defineEmits<{ submit: [date: string] }>()
const open = defineModel<boolean>('open', { required: true })

const { t } = useI18n()
const settings = useSettingsStore()
const date = ref<string | null>(todayIso())

watch(open, (v) => {
  if (v) date.value = todayIso()
})

const count = computed(() => (date.value === null ? 0 : props.captures.filter((c) => c.date <= date.value!).length))

function submit() {
  if (date.value === null || count.value === 0) return
  emit('submit', date.value)
  open.value = false
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-md">
      <DialogHeader>
        <DialogTitle>{{ t('inbox.ignoreUntilTitle') }}</DialogTitle>
        <DialogDescription data-testid="ignore-until-hint" aria-live="polite">
          <template v-if="date">{{ t('inbox.ignoreUntilHint', { n: count, date: formatDateInput(date, settings.locale) }) }}</template>
        </DialogDescription>
      </DialogHeader>
      <form class="flex flex-col gap-4" novalidate @submit.prevent="submit">
        <div class="flex flex-col gap-2">
          <Label for="ignore-until-date">{{ t('inbox.ignoreUntilDate') }}</Label>
          <DateInput id="ignore-until-date" v-model="date" />
        </div>
        <DialogFooter>
          <Button type="button" variant="ghost" @click="open = false">{{ t('actions.cancel') }}</Button>
          <Button type="submit" data-testid="ignore-until-submit" :disabled="count === 0">{{ t('inbox.ignore') }}</Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
