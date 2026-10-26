<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from 'vue-sonner'
import TxnForm from '@/components/txn/TxnForm.vue'
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet'
import * as ipc from '@/ipc'
import type { RecurrenceDto } from '@/ipc'
import { defaultAccountId } from '@/lib/accounts'
import { todayIso } from '@/lib/date'
import { duplicateOf, emptyForm, fromDto, type TxnFormState } from '@/lib/txnForm'
import { withErrorToast } from '@/lib/notify'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'
import { useScheduledStore } from '@/stores/scheduled'
import { useUiStore, type RuleEditor } from '@/stores/ui'
import RecurrenceFields from './RecurrenceFields.vue'

// Sheet jadwal berulang (K14), pola sama dengan NewTxnSheet: remount form tiap dibuka.
const ui = useUiStore()
const accounts = useAccountsStore()
const categories = useCategoriesStore()
const scheduled = useScheduledStore()
const { t } = useI18n()

const initial = ref<TxnFormState | null>(null)
const rec = ref<RecurrenceDto>({ freq: 'monthly', interval: 1, until: null })
const autoPost = ref(false)
const session = ref(0)
let returnFocus: HTMLElement | null = null

const open = computed({
  get: () => ui.ruleEditor !== null,
  set: (v) => { if (!v) ui.closeRuleEditor() },
})

const mode = computed(() => ui.ruleEditor?.mode ?? 'create')
const title = computed(() => t(`scheduled.title.${mode.value}`))
const dateLabel = computed(() => {
  if (mode.value === 'occurrence') return undefined
  return t(mode.value === 'edit' ? 'scheduled.upcoming' : 'scheduled.start')
})

async function findRule(id: string) {
  await scheduled.reload()
  const rule = scheduled.rules.find((r) => r.id === id)
  if (!rule) throw new ipc.IpcFailure({ code: 'not_found', entity: 'recurring_rule' })
  return rule
}

async function prepare(ed: RuleEditor) {
  await Promise.all([accounts.ensureLoaded(), categories.ensureLoaded()])
  let state: TxnFormState
  rec.value = { freq: 'monthly', interval: 1, until: null }
  autoPost.value = false
  if (ed.mode === 'create') {
    const defaults = await ipc.txnFormDefaults()
    state = emptyForm('expense', { accountId: defaultAccountId(accounts.overview.accounts, defaults.lastAccountId), feeCategoryId: defaults.feeCategoryId, today: todayIso() })
  } else if (ed.mode === 'fromTxn') {
    const d = await ipc.getTxn(ed.txnId)
    // Kejadian pertama = transaksi asal yang sudah tercatat; jadwal mulai dari kejadian kedua.
    const [, second] = await ipc.previewRecurrence(rec.value, d.date, 2)
    state = duplicateOf(d, second ?? d.date)
  } else {
    const rule = await findRule(ed.mode === 'edit' ? ed.id : ed.ruleId)
    state = fromDto(rule.form)
    if (ed.mode === 'occurrence') state.date = ed.date
    rec.value = { ...rule.recurrence }
    autoPost.value = rule.autoPost
  }
  if (ui.ruleEditor !== ed) return
  initial.value = state
  session.value++
}

watch(() => ui.ruleEditor, (ed) => {
  initial.value = null
  if (!ed) return
  if (document.activeElement instanceof HTMLElement && document.activeElement !== document.body) returnFocus = document.activeElement
  void withErrorToast(async () => {
    try {
      await prepare(ed)
    } catch (e) {
      ui.closeRuleEditor()
      throw e
    }
  })
}, { immediate: true })

async function submit(s: TxnFormState): Promise<string> {
  const ed = ui.ruleEditor
  switch (ed?.mode) {
    case 'edit':
      await scheduled.update(ed.id, s, rec.value, autoPost.value)
      return ed.id
    case 'occurrence':
      return scheduled.confirmEdited(ed.ruleId, ed.date, s)
    default:
      return scheduled.create(s, rec.value, autoPost.value)
  }
}

function saved() {
  toast.success(t(mode.value === 'occurrence' ? 'scheduled.recorded' : 'scheduled.saved'))
  ui.closeRuleEditor()
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
    <SheetContent side="right" class="w-full gap-0 sm:max-w-[27.5rem]" @open-auto-focus.prevent @close-auto-focus="restoreFocus">
      <SheetHeader>
        <SheetTitle>{{ title }}</SheetTitle>
        <SheetDescription class="sr-only">{{ title }}</SheetDescription>
      </SheetHeader>
      <TxnForm
        v-if="initial"
        :key="session"
        :initial="initial"
        :id="null"
        :submit="submit"
        :date-label="dateLabel"
        @saved="saved"
        @cancel="ui.closeRuleEditor()"
      >
        <template v-if="mode !== 'occurrence'" #extra="{ state }">
          <RecurrenceFields v-model="rec" v-model:auto-post="autoPost" :start="state.date" />
        </template>
      </TxnForm>
    </SheetContent>
  </Sheet>
</template>
