<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from 'vue-sonner'
import TxnForm from '@/components/txn/TxnForm.vue'
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet'
import * as ipc from '@/ipc'
import { isLiability } from '@/lib/accounts'
import { todayIso } from '@/lib/date'
import { duplicateOf, emptyForm, fromDetail, type TxnFormState } from '@/lib/txnForm'
import { withErrorToast } from '@/lib/notify'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'
import { useUiStore } from '@/stores/ui'

const ui = useUiStore()
const accounts = useAccountsStore()
const categories = useCategoriesStore()
const { t } = useI18n()

const initial = ref<TxnFormState | null>(null)
// Remount form setiap kali sheet dibuka agar state tidak bocor antar sesi.
const session = ref(0)
let returnFocus: HTMLElement | null = null

const open = computed({
  get: () => ui.txnEditor !== null,
  set: (v) => { if (!v) ui.closeTxnEditor() },
})

const title = computed(() => t(`txn.title.${ui.txnEditor?.mode ?? 'create'}`))

function defaultAccount(last: string | null): string | null {
  const active = accounts.overview.accounts.filter((a) => !a.archived)
  if (last && active.some((a) => a.id === last)) return last
  return (active.find((a) => !isLiability(a.kind)) ?? active[0])?.id ?? null
}

async function prepare() {
  const ed = ui.txnEditor
  if (!ed) return
  await Promise.all([accounts.loaded ? null : accounts.load(), categories.loaded ? null : categories.load()])
  const defaults = await ipc.txnFormDefaults()
  let state: TxnFormState
  if (ed.mode === 'create' || !ed.id) {
    state = emptyForm('expense', { accountId: defaultAccount(defaults.lastAccountId), feeCategoryId: defaults.feeCategoryId, today: todayIso() })
  } else {
    const d = await ipc.getTxn(ed.id)
    state = ed.mode === 'edit' ? fromDetail(d) : duplicateOf(d, todayIso())
    state.feeCategoryId ??= defaults.feeCategoryId
  }
  if (ui.txnEditor !== ed) return
  initial.value = state
  session.value++
}

watch(() => ui.txnEditor, (ed) => {
  initial.value = null
  if (!ed) return
  if (document.activeElement instanceof HTMLElement && document.activeElement !== document.body) returnFocus = document.activeElement
  void withErrorToast(async () => {
    try {
      await prepare()
    } catch (e) {
      ui.closeTxnEditor()
      throw e
    }
  })
}, { immediate: true })

function saved() {
  toast.success(t('txn.saved'))
  ui.closeTxnEditor()
}

function restoreFocus(e: Event) {
  if (!returnFocus) return
  e.preventDefault()
  returnFocus.focus()
  returnFocus = null
}
</script>

<template>
  <Sheet v-model:open="open">
    <SheetContent side="right" class="w-full gap-0 sm:max-w-[440px]" @open-auto-focus.prevent @close-auto-focus="restoreFocus">
      <SheetHeader>
        <SheetTitle>{{ title }}</SheetTitle>
        <SheetDescription class="sr-only">{{ title }}</SheetDescription>
      </SheetHeader>
      <TxnForm
        v-if="initial"
        :key="session"
        :initial="initial"
        :id="ui.txnEditor?.mode === 'edit' ? ui.txnEditor.id ?? null : null"
        @saved="saved"
        @cancel="ui.closeTxnEditor()"
      />
    </SheetContent>
  </Sheet>
</template>
