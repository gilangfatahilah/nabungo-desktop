<script setup lang="ts">
import { FileTextIcon, ImageIcon, PaperclipIcon, Trash2Icon } from '@lucide/vue'
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import { Button } from '@/components/ui/button'
import { addAttachments, listAttachments, openAttachment, removeAttachment, type AttachmentDto } from '@/ipc'
import { formatBytes } from '@/lib/bytes'
import { errorMessage, type Translate } from '@/lib/errors'
import { useSettingsStore } from '@/stores/settings'
import { useTransactionsStore } from '@/stores/transactions'

// Lampiran struk (TRX-08). Hanya untuk transaksi tersimpan; file dikelola backend di folder data.
const props = defineProps<{ txnId: string | null }>()
const emit = defineEmits<{ loaded: [count: number] }>()

const { t } = useI18n()
const settings = useSettingsStore()
const txns = useTransactionsStore()

const items = ref<AttachmentDto[]>([])
const error = ref<string | null>(null)
const busy = ref(false)

async function run(fn: () => Promise<unknown>) {
  busy.value = true
  error.value = null
  try {
    await fn()
  } catch (e) {
    error.value = errorMessage(e, t as unknown as Translate)
  } finally {
    busy.value = false
  }
}

async function load() {
  if (!props.txnId) return
  items.value = await listAttachments(props.txnId)
  emit('loaded', items.value.length)
}

watch(() => props.txnId, () => void run(load), { immediate: true })

// Daftar transaksi ikut dimuat ulang agar ikon klip di baris sesuai.
const add = () => run(async () => {
  await addAttachments(props.txnId!)
  await load()
  await txns.reload()
})

const open = (a: AttachmentDto) => run(() => openAttachment(a.id))

const pending = ref<AttachmentDto | null>(null)
const confirmOpen = ref(false)

function askRemove(a: AttachmentDto) {
  pending.value = a
  confirmOpen.value = true
}

function onConfirm() {
  const a = pending.value
  if (!a) return
  void run(async () => {
    await removeAttachment(a.id)
    await load()
    await txns.reload()
  })
}
</script>

<template>
  <div data-testid="txn-attachments" class="flex flex-col gap-2">
    <p class="text-sm font-medium">{{ t('txn.attachments.title') }}</p>
    <p v-if="!txnId" class="text-meta text-muted-foreground">{{ t('txn.attachments.saveFirst') }}</p>
    <template v-else>
      <ul v-if="items.length" class="flex flex-col">
        <li v-for="a in items" :key="a.id" data-testid="attachment-row" class="flex h-9 items-center gap-2">
          <component :is="a.mime === 'application/pdf' ? FileTextIcon : ImageIcon" class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
          <span class="min-w-0 flex-1 truncate" :class="a.missing && 'text-muted-foreground line-through'">{{ a.fileName }}</span>
          <span v-if="a.missing" class="shrink-0 text-meta text-destructive">{{ t('txn.attachments.missing') }}</span>
          <span class="num shrink-0 text-meta text-muted-foreground">{{ formatBytes(a.sizeBytes, settings.locale) }}</span>
          <Button type="button" variant="ghost" size="sm" :disabled="a.missing || busy" @click="open(a)">{{ t('txn.attachments.open') }}</Button>
          <Button
            type="button" variant="ghost" size="icon" class="size-8" data-testid="attachment-remove"
            :disabled="busy" :aria-label="t('txn.attachments.removeTitle', { name: a.fileName })" @click="askRemove(a)"
          >
            <Trash2Icon />
          </Button>
        </li>
      </ul>
      <Button type="button" variant="outline" size="sm" class="self-start" :disabled="busy" @click="add">
        <PaperclipIcon /> {{ t('txn.attachments.add') }}
      </Button>
    </template>
    <p v-if="error" class="text-meta text-destructive" role="alert">{{ error }}</p>
    <ConfirmDialog
      v-if="pending"
      v-model:open="confirmOpen"
      :title="t('txn.attachments.removeTitle', { name: pending.fileName })"
      :description="t('txn.attachments.removeBody')"
      :confirm-label="t('actions.delete')"
      destructive
      @confirm="onConfirm"
    />
  </div>
</template>
