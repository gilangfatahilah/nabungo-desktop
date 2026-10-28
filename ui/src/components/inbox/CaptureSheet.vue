<script setup lang="ts">
import { ChevronRight } from '@lucide/vue'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from 'vue-sonner'
import DuplicateList from '@/components/inbox/DuplicateList.vue'
import TxnForm from '@/components/txn/TxnForm.vue'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet'
import * as ipc from '@/ipc'
import type { CaptureDto } from '@/ipc'
import { fromCapture } from '@/lib/capture'
import { withErrorToast } from '@/lib/notify'
import type { TxnFormState } from '@/lib/txnForm'
import { cn } from '@/lib/utils'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'
import { useInboxStore } from '@/stores/inbox'

// Tinjau satu draft email sebelum dicatat (EML-03). Pola sheet mengikuti NewTxnSheet.
const props = defineProps<{ capture: CaptureDto | null }>()
const emit = defineEmits<{ close: [] }>()

const { t } = useI18n()
const inbox = useInboxStore()
const accounts = useAccountsStore()
const categories = useCategoriesStore()

// K11: snapshot saat dibuka; reload daftar (sync, aksi lain) tidak menimpa isian user.
const snapshot = ref<CaptureDto | null>(null)
const initial = ref<TxnFormState | null>(null)
// Remount form setiap kali sheet dibuka agar state tidak bocor antar sesi.
const session = ref(0)
const linking = ref(false)
const snippetOpen = ref(false)

const open = computed({
  get: () => props.capture !== null,
  set: (v) => { if (!v) emit('close') },
})

async function prepare(c: CaptureDto) {
  const [defaults] = await Promise.all([ipc.txnFormDefaults(), accounts.ensureLoaded(), categories.ensureLoaded()])
  if (props.capture?.id !== c.id) return
  initial.value = fromCapture(c, { accounts: accounts.overview.accounts, categories: categories.list, feeCategoryId: defaults.feeCategoryId })
  session.value++
}

watch(() => props.capture?.id ?? null, (id) => {
  if (id === null) {
    snapshot.value = null
    initial.value = null
    return
  }
  if (snapshot.value?.id === id) return
  snapshot.value = props.capture
  initial.value = null
  snippetOpen.value = false
  void withErrorToast(async () => {
    try {
      await prepare(props.capture!)
    } catch (e) {
      emit('close')
      throw e
    }
  })
}, { immediate: true })

function submit(s: TxnFormState): Promise<string> {
  return inbox.confirm(snapshot.value!.id, s)
}

function saved() {
  toast.success(t('inbox.recorded'))
  emit('close')
}

async function link(txnId: string) {
  linking.value = true
  try {
    await withErrorToast(async () => {
      await inbox.link(snapshot.value!.id, txnId)
      toast.success(t('inbox.linked'))
      emit('close')
    })
  } finally {
    linking.value = false
  }
}
</script>

<template>
  <Sheet v-model:open="open">
    <SheetContent side="right" class="w-full gap-0 sm:max-w-[27.5rem]" @open-auto-focus.prevent>
      <SheetHeader>
        <SheetTitle>{{ t('inbox.reviewTitle') }}</SheetTitle>
        <SheetDescription class="sr-only">{{ snapshot?.ruleName }}</SheetDescription>
      </SheetHeader>
      <div v-if="snapshot && (snapshot.snippet || snapshot.duplicates.length)" class="flex flex-col gap-3 px-4 pb-3">
        <section v-if="snapshot.duplicates.length" class="rounded-lg border border-warning/50 px-3 pt-2">
          <h3 class="text-meta font-medium text-warning">{{ t('inbox.duplicatesHint') }}</h3>
          <DuplicateList :duplicates="snapshot.duplicates" :busy="linking" @link="link" />
        </section>
        <Collapsible v-if="snapshot.snippet" v-model:open="snippetOpen">
          <CollapsibleTrigger
            class="flex items-center gap-1 rounded-md text-meta font-medium text-muted-foreground hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
          >
            <ChevronRight :class="cn('size-4 transition-transform', snippetOpen && 'rotate-90')" aria-hidden="true" />
            {{ t('inbox.originalEmail') }}
          </CollapsibleTrigger>
          <CollapsibleContent>
            <p class="mt-2 max-h-40 overflow-y-auto rounded-md bg-muted px-3 py-2 text-meta whitespace-pre-line text-muted-foreground">{{ snapshot.snippet }}</p>
          </CollapsibleContent>
        </Collapsible>
      </div>
      <TxnForm
        v-if="initial"
        :key="session"
        :initial="initial"
        :id="null"
        :submit="submit"
        @saved="saved"
        @cancel="emit('close')"
      />
    </SheetContent>
  </Sheet>
</template>
