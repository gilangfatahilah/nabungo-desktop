<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from 'vue-sonner'
import DuplicateList from '@/components/inbox/DuplicateList.vue'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import type { CaptureDto } from '@/ipc'
import { withErrorToast } from '@/lib/notify'
import { useInboxStore } from '@/stores/inbox'

// Hubungkan draft email ke transaksi yang sudah dicatat manual (tidak membuat transaksi baru).
const props = defineProps<{ capture: CaptureDto | null }>()
const emit = defineEmits<{ close: [] }>()

const { t } = useI18n()
const inbox = useInboxStore()
const busy = ref(false)

const open = computed({
  get: () => props.capture !== null,
  set: (v) => { if (!v) emit('close') },
})

async function link(txnId: string) {
  const c = props.capture
  if (!c) return
  busy.value = true
  try {
    await withErrorToast(async () => {
      await inbox.link(c.id, txnId)
      toast.success(t('inbox.linked'))
      emit('close')
    })
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-md">
      <DialogHeader>
        <DialogTitle>{{ t('inbox.duplicatesTitle') }}</DialogTitle>
        <DialogDescription>{{ t('inbox.duplicatesHint') }}</DialogDescription>
      </DialogHeader>
      <DuplicateList v-if="capture" :duplicates="capture.duplicates" :busy="busy" @link="link" />
    </DialogContent>
  </Dialog>
</template>
