<script setup lang="ts">
import { ChevronRight } from '@lucide/vue'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { toast } from 'vue-sonner'
import RuleTestPanel from '@/components/inbox/RuleTestPanel.vue'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet'
import { Switch } from '@/components/ui/switch'
import * as ipc from '@/ipc'
import type { EmailRuleDto, EmailRuleInputDto, EmailRuleKindDto } from '@/ipc'
import { withErrorToast } from '@/lib/notify'
import { cn } from '@/lib/utils'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'
import { useEmailRulesStore } from '@/stores/emailRules'
import { useInboxStore } from '@/stores/inbox'

// Buat/ubah rule email capture (EML-04, docs/design/design.md §8.10). Validasi pola & referensi di Rust;
// error `email_rule_invalid` ditampilkan di field-nya.
const props = defineProps<{
  open: boolean
  rule: EmailRuleDto | null
  seed: { input: EmailRuleInputDto; sampleMessageId: string | null } | null
}>()
const emit = defineEmits<{ close: [] }>()

const { t } = useI18n()
const accounts = useAccountsStore()
const categories = useCategoriesStore()
const rules = useEmailRulesStore()
const inbox = useInboxStore()

const KINDS: EmailRuleKindDto[] = ['expense', 'income', 'transfer_out', 'transfer_in']
// `nabungo_mail::rule::DATE_FORMATS`.
const DATE_FORMATS = ['dd/mm/yyyy', 'dd-mm-yyyy', 'dd mmm yyyy']
// Reka Select tidak menerima nilai kosong.
const NONE = '__none'
const PATTERN_FIELDS = new Set(['subject', 'amount', 'payee', 'date', 'date_format'])

type Form = Omit<EmailRuleInputDto, 'priority'> & { priority: string }

const f = ref<Form>(blank())
const patternsOpen = ref(true)
const fieldError = ref<string | null>(null)
const busy = ref(false)
// Remount panel uji setiap kali sheet dibuka agar hasil uji lama tidak terbawa.
const session = ref(0)

function blank(): Form {
  return {
    name: '', senderPattern: '', subjectRegex: null, amountRegex: '', payeeRegex: null, dateRegex: null, dateFormat: null,
    kind: 'expense', accountId: null, counterAccountId: null, defaultCategoryId: null, priority: '10', enabled: true,
  }
}

const open = computed({
  get: () => props.open,
  set: (v) => { if (!v) emit('close') },
})

// Snapshot saat dibuka: reload daftar rule/inbox tidak menimpa isian.
watch(() => props.open, (v) => {
  if (!v) return
  const src = props.rule ?? props.seed?.input
  f.value = src ? { ...src, priority: String(src.priority) } : blank()
  // K10: pola terbuka untuk rule baru/dari email, tertutup saat mengubah.
  patternsOpen.value = props.rule === null
  fieldError.value = null
  session.value++
  void withErrorToast(() => Promise.all([accounts.ensureLoaded(), categories.ensureLoaded()]))
}, { immediate: true })

const transfer = computed(() => f.value.kind === 'transfer_out' || f.value.kind === 'transfer_in')
const accountOptions = computed(() => accounts.overview.accounts.filter((a) => !a.archived))
const categoryOptions = computed(() =>
  categories.list.filter((c) => !c.archived && c.kind === f.value.kind),
)

/** Ganti jenis: field yang tidak berlaku dikosongkan, kategori beda jenis juga. */
function setKind(k: EmailRuleKindDto) {
  f.value.kind = k
  if (transfer.value) {
    f.value.defaultCategoryId = null
    return
  }
  f.value.counterAccountId = null
  const c = categories.list.find((x) => x.id === f.value.defaultCategoryId)
  if (c && c.kind !== k) f.value.defaultCategoryId = null
}

const select = (v: string | null) => (v === null ? NONE : v)
const unselect = (v: unknown) => (v === NONE || typeof v !== 'string' ? null : v)
const opt = (v: string | null) => (v === null || v.trim() === '' ? null : v)

function payload(): EmailRuleInputDto {
  const x = f.value
  const priority = Number.parseInt(x.priority, 10)
  return {
    ...x, subjectRegex: opt(x.subjectRegex), payeeRegex: opt(x.payeeRegex), dateRegex: opt(x.dateRegex), dateFormat: opt(x.dateFormat),
    priority: Number.isFinite(priority) ? priority : 0,
  }
}
// Panel uji memakai isian terkini.
const testInput = computed(payload)

async function submit() {
  if (busy.value) return
  busy.value = true
  fieldError.value = null
  try {
    await withErrorToast(async () => {
      try {
        const saved = await rules.save(props.rule?.id ?? null, payload())
        toast.success(saved.reparsed.emails > 0 ? t('emailRules.savedReparsed', { n: saved.reparsed.emails }) : t('emailRules.saved'))
        emit('close')
      } catch (e) {
        if (!(e instanceof ipc.IpcFailure) || e.error.code !== 'email_rule_invalid' || !e.error.entity) throw e
        fieldError.value = e.error.entity
        if (PATTERN_FIELDS.has(e.error.entity)) patternsOpen.value = true
      }
    })
  } finally {
    busy.value = false
  }
}

const invalid = (field: string) => fieldError.value === field
const describedBy = (field: string, hint?: string) =>
  [invalid(field) ? `er-error-${field}` : null, hint ?? null].filter(Boolean).join(' ') || undefined
</script>

<template>
  <Sheet v-model:open="open">
    <SheetContent side="right" class="w-full gap-0 sm:max-w-[27.5rem]" @open-auto-focus.prevent>
      <SheetHeader>
        <SheetTitle>{{ rule ? t('emailRules.edit') : t('emailRules.new') }}</SheetTitle>
        <SheetDescription class="sr-only">{{ f.name }}</SheetDescription>
      </SheetHeader>
      <form data-testid="er-form" class="flex min-h-0 flex-1 flex-col" novalidate @submit.prevent="submit" @keydown.ctrl.enter.prevent="submit" @keydown.meta.enter.prevent="submit">
        <div class="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-4 pt-1 pb-4">
          <div class="flex flex-col gap-2">
            <Label for="er-name">{{ t('emailRules.name') }}</Label>
            <Input id="er-name" v-model="f.name" :aria-invalid="invalid('name') || undefined" :aria-describedby="describedBy('name')" />
            <p v-if="invalid('name')" id="er-error-name" data-testid="er-error-name" class="text-meta text-destructive">{{ t('emailRules.fieldError.name') }}</p>
          </div>

          <div class="flex flex-col gap-2">
            <Label id="er-kind-label">{{ t('emailRules.kindLabel') }}</Label>
            <Select :model-value="f.kind" @update:model-value="setKind($event as EmailRuleKindDto)">
              <SelectTrigger aria-labelledby="er-kind-label" class="w-full" data-testid="er-kind"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem v-for="k in KINDS" :key="k" :value="k">{{ t(`emailRules.kind.${k}`) }}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div class="flex flex-col gap-2">
            <Label for="er-sender">{{ t('emailRules.sender') }}</Label>
            <Input
              id="er-sender" v-model="f.senderPattern" autocomplete="off"
              :aria-invalid="invalid('sender') || undefined" :aria-describedby="describedBy('sender', 'er-sender-hint')"
            />
            <p id="er-sender-hint" class="text-meta text-muted-foreground">{{ t('emailRules.senderHint') }}</p>
            <p v-if="invalid('sender')" id="er-error-sender" data-testid="er-error-sender" class="text-meta text-destructive">{{ t('emailRules.fieldError.sender') }}</p>
          </div>

          <div class="flex flex-col gap-2">
            <Label id="er-account-label">{{ t('emailRules.account') }}</Label>
            <Select :model-value="select(f.accountId)" @update:model-value="f.accountId = unselect($event)">
              <SelectTrigger aria-labelledby="er-account-label" class="w-full" data-testid="er-account"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem :value="NONE">{{ t('emailRules.noAccount') }}</SelectItem>
                <SelectItem v-for="a in accountOptions" :key="a.id" :value="a.id">{{ a.name }}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div v-if="transfer" class="flex flex-col gap-2">
            <Label id="er-counter-label">{{ t('emailRules.counterAccount') }}</Label>
            <Select :model-value="select(f.counterAccountId)" @update:model-value="f.counterAccountId = unselect($event)">
              <SelectTrigger
                aria-labelledby="er-counter-label" class="w-full" data-testid="er-counter"
                :aria-invalid="invalid('counter_account') || undefined" :aria-describedby="describedBy('counter_account')"
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem :value="NONE">{{ t('emailRules.noAccount') }}</SelectItem>
                <SelectItem v-for="a in accountOptions" :key="a.id" :value="a.id" :disabled="a.id === f.accountId">{{ a.name }}</SelectItem>
              </SelectContent>
            </Select>
            <p v-if="invalid('counter_account')" id="er-error-counter_account" class="text-meta text-destructive">{{ t('emailRules.fieldError.counter_account') }}</p>
          </div>
          <div v-else class="flex flex-col gap-2">
            <Label id="er-category-label">{{ t('emailRules.category') }}</Label>
            <Select :model-value="select(f.defaultCategoryId)" @update:model-value="f.defaultCategoryId = unselect($event)">
              <SelectTrigger aria-labelledby="er-category-label" class="w-full" data-testid="er-category"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem :value="NONE">{{ t('emailRules.noCategory') }}</SelectItem>
                <SelectItem v-for="c in categoryOptions" :key="c.id" :value="c.id">{{ c.name }}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <Collapsible v-model:open="patternsOpen" class="flex flex-col gap-4">
            <CollapsibleTrigger
              class="flex items-center gap-1 self-start rounded-md text-meta font-medium text-muted-foreground hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
            >
              <ChevronRight :class="cn('size-4 transition-transform', patternsOpen && 'rotate-90')" aria-hidden="true" />
              {{ t('emailRules.patterns') }}
            </CollapsibleTrigger>
            <CollapsibleContent class="flex flex-col gap-4">
              <p id="er-regex-hint" class="text-meta text-muted-foreground">{{ t('emailRules.regexHint') }}</p>
              <div v-for="p in ([
                { field: 'subject', key: 'subjectRegex' },
                { field: 'amount', key: 'amountRegex' },
                { field: 'payee', key: 'payeeRegex' },
                { field: 'date', key: 'dateRegex' },
              ] as const)" :key="p.field" class="flex flex-col gap-2">
                <Label :for="`er-${p.field}`">{{ t(`emailRules.${p.field}`) }}</Label>
                <Input
                  :id="`er-${p.field}`" :model-value="f[p.key] ?? ''" class="font-mono text-meta" autocomplete="off" spellcheck="false"
                  :aria-invalid="invalid(p.field) || undefined" :aria-describedby="describedBy(p.field, 'er-regex-hint')"
                  @update:model-value="f[p.key] = String($event)"
                />
                <p v-if="invalid(p.field)" :id="`er-error-${p.field}`" :data-testid="`er-error-${p.field}`" class="text-meta text-destructive">
                  {{ t(`emailRules.fieldError.${p.field}`) }}
                </p>
              </div>
              <div class="flex flex-col gap-2">
                <Label id="er-date-format-label">{{ t('emailRules.dateFormat') }}</Label>
                <Select :model-value="select(f.dateFormat)" @update:model-value="f.dateFormat = unselect($event)">
                  <SelectTrigger
                    aria-labelledby="er-date-format-label" class="w-full" data-testid="er-date-format"
                    :aria-invalid="invalid('date_format') || undefined" :aria-describedby="describedBy('date_format')"
                  >
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem :value="NONE">{{ t('emailRules.noFormat') }}</SelectItem>
                    <SelectItem v-for="d in DATE_FORMATS" :key="d" :value="d">{{ d }}</SelectItem>
                  </SelectContent>
                </Select>
                <p v-if="invalid('date_format')" id="er-error-date_format" data-testid="er-error-date_format" class="text-meta text-destructive">
                  {{ t('emailRules.fieldError.date_format') }}
                </p>
              </div>
            </CollapsibleContent>
          </Collapsible>

          <div class="grid grid-cols-[1fr_auto] items-end gap-4">
            <div class="flex flex-col gap-2">
              <Label for="er-priority">{{ t('emailRules.priority') }}</Label>
              <Input id="er-priority" v-model="f.priority" inputmode="numeric" class="num w-24" aria-describedby="er-priority-hint" />
            </div>
            <div class="flex h-9 items-center gap-2">
              <Switch id="er-enabled" v-model="f.enabled" />
              <Label for="er-enabled">{{ t('emailRules.enabled') }}</Label>
            </div>
            <p id="er-priority-hint" class="col-span-2 text-meta text-muted-foreground">{{ t('emailRules.priorityHint') }}</p>
          </div>

          <RuleTestPanel :key="session" :input="testInput" :unparsed="inbox.unparsed" :initial-message-id="seed?.sampleMessageId ?? null" />
        </div>

        <div class="flex justify-end gap-2 border-t px-4 py-3">
          <Button type="button" variant="ghost" @click="emit('close')">{{ t('actions.cancel') }}</Button>
          <Button type="submit" :disabled="busy">{{ t('actions.save') }}</Button>
        </div>
      </form>
    </SheetContent>
  </Sheet>
</template>
