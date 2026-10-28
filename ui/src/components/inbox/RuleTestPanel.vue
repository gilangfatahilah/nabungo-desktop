<script setup lang="ts">
import { Check, X } from '@lucide/vue'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import MoneyText from '@/components/MoneyText.vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Textarea } from '@/components/ui/textarea'
import * as ipc from '@/ipc'
import type { EmailRuleInputDto, RuleSampleDto, RuleTestDto, UnparsedEmailDto } from '@/ipc'
import { formatDateInput } from '@/lib/date'
import { errorMessage, type Translate } from '@/lib/errors'
import { useSettingsStore } from '@/stores/settings'

// Uji rule terhadap satu email (EML-04, K5): langkah yang lolos/gagal, lalu hasilnya. Tidak menulis apa pun;
// teks yang ditempel hanya dikirim ke `test_email_rule` dan tidak disimpan.
const props = defineProps<{ input: EmailRuleInputDto; unparsed: UnparsedEmailDto[]; initialMessageId?: string | null }>()

const { t } = useI18n()
const settings = useSettingsStore()

const initial = props.unparsed.find((u) => u.messageId === props.initialMessageId) ?? props.unparsed.find((u) => u.sender !== '')
const mode = ref<'unparsed' | 'pasted'>(initial ? 'unparsed' : 'pasted')
const messageId = ref(initial?.messageId ?? '')
const pasted = ref({ sender: '', subject: '', text: '' })
const result = ref<RuleTestDto | null>(null)
const error = ref<string | null>(null)
const running = ref(false)

// Email tanpa pengirim tidak bisa dibaca rule mana pun.
const samples = computed(() => props.unparsed.filter((u) => u.sender !== ''))

function sample(): RuleSampleDto {
  return mode.value === 'unparsed'
    ? { kind: 'unparsed', messageId: messageId.value }
    : { kind: 'pasted', ...pasted.value }
}

async function run() {
  if (mode.value === 'unparsed' && !messageId.value) return
  running.value = true
  error.value = null
  try {
    result.value = await ipc.testEmailRule(props.input, sample())
  } catch (e) {
    result.value = null
    const code = e instanceof ipc.IpcFailure ? e.error : null
    error.value = code?.code === 'email_rule_invalid' && code.entity
      ? t(`emailRules.fieldError.${code.entity}`)
      : errorMessage(e, t as unknown as Translate)
  } finally {
    running.value = false
  }
}

onMounted(() => {
  if (props.initialMessageId && mode.value === 'unparsed') void run()
})

type Step = { id: string; label: string; ok: boolean; detail: string | null }

/** Langkah yang dijalankan rule engine, sampai langkah yang gagal (urutan `nabungo_mail::rule::trace`). */
const steps = computed<Step[]>(() => {
  const r = result.value
  if (!r) return []
  const out: Step[] = [{ id: 'sender', label: t('emailRules.stepSender'), ok: r.senderOk, detail: null }]
  if (!r.senderOk) return out
  if (r.subjectOk !== null) out.push({ id: 'subject', label: t('emailRules.stepSubject'), ok: r.subjectOk, detail: null })
  if (r.fail === 'subject') return out
  const amountOk = r.fail !== 'no_amount' && r.fail !== 'amount_invalid'
  out.push({ id: 'amount', label: t('emailRules.stepAmount'), ok: amountOk, detail: r.amounts.join(' · ') || null })
  if (r.fail === 'no_amount') return out
  if (r.payee !== null) out.push({ id: 'payee', label: t('emailRules.stepPayee'), ok: true, detail: r.payee })
  out.push({
    id: 'date', label: t('emailRules.stepDate'), ok: r.fail !== 'no_date' && r.fail !== 'date_invalid',
    detail: r.dateFromHeader ? t('emailRules.dateFromHeader') : r.dateText,
  })
  return out
})

const sign = computed(() => (props.input.kind === 'expense' ? -1 : 1))
const tone = computed(() => (props.input.kind === 'expense' || props.input.kind === 'income' ? props.input.kind : 'neutral'))
</script>

<template>
  <section class="flex flex-col gap-3 rounded-lg border px-3 py-3" :aria-label="t('emailRules.testTitle')">
    <h3 class="text-meta font-medium">{{ t('emailRules.testTitle') }}</h3>
    <Tabs v-model="mode">
      <TabsList class="w-full">
        <TabsTrigger value="unparsed" :disabled="!samples.length">{{ t('emailRules.sampleUnparsed') }}</TabsTrigger>
        <TabsTrigger value="pasted">{{ t('emailRules.samplePasted') }}</TabsTrigger>
      </TabsList>
    </Tabs>

    <div v-if="mode === 'unparsed'" class="flex flex-col gap-2">
      <Label id="er-sample-label">{{ t('emailRules.sampleEmail') }}</Label>
      <Select v-model="messageId">
        <SelectTrigger aria-labelledby="er-sample-label" class="w-full" data-testid="er-sample"><SelectValue /></SelectTrigger>
        <SelectContent>
          <SelectItem v-for="u in samples" :key="u.messageId" :value="u.messageId">{{ u.subject || t('inbox.noSubject') }} · {{ u.sender }}</SelectItem>
        </SelectContent>
      </Select>
    </div>
    <template v-else>
      <div class="flex flex-col gap-2">
        <Label for="er-sample-sender">{{ t('emailRules.sender') }}</Label>
        <Input id="er-sample-sender" v-model="pasted.sender" type="email" autocomplete="off" />
      </div>
      <div class="flex flex-col gap-2">
        <Label for="er-sample-subject">{{ t('emailRules.sampleSubject') }}</Label>
        <Input id="er-sample-subject" v-model="pasted.subject" autocomplete="off" />
      </div>
      <div class="flex flex-col gap-2">
        <Label for="er-sample-text">{{ t('emailRules.pastedText') }}</Label>
        <Textarea id="er-sample-text" v-model="pasted.text" rows="5" class="font-mono text-meta" />
      </div>
    </template>

    <Button type="button" variant="outline" size="sm" class="self-start" :disabled="running" @click="run">{{ t('emailRules.test') }}</Button>

    <p v-if="error" class="text-meta text-destructive" role="alert">{{ error }}</p>
    <div v-if="result" class="flex flex-col gap-2" role="status">
      <ol data-testid="rule-trace" class="flex flex-col gap-1 text-meta">
        <li v-for="s in steps" :key="s.id" :data-step="s.id" :data-ok="String(s.ok)" class="flex min-w-0 items-center gap-2">
          <Check v-if="s.ok" class="size-4 shrink-0 text-income" aria-hidden="true" />
          <X v-else class="size-4 shrink-0 text-destructive" aria-hidden="true" />
          <span class="sr-only">{{ s.ok ? '✓' : '✗' }}</span>
          <span class="shrink-0">{{ s.label }}</span>
          <span v-if="s.detail" class="num truncate text-muted-foreground">{{ s.detail }}</span>
        </li>
      </ol>
      <p v-if="result.fail" data-testid="rule-test-fail" class="text-meta text-destructive">
        {{ t(`emailRules.fail.${result.fail}`) }} <span v-if="result.failText" class="num">{{ result.failText }}</span>
      </p>
      <ul v-else data-testid="rule-test-items" class="flex flex-col gap-1">
        <li v-for="(it, i) in result.items" :key="i" class="flex min-w-0 items-center gap-3 rounded-md bg-muted px-2 py-1.5">
          <span class="num shrink-0 text-meta text-muted-foreground">{{ formatDateInput(it.date, settings.locale) }}</span>
          <span class="min-w-0 flex-1 truncate">{{ it.payee ?? '' }}</span>
          <MoneyText :amount="sign * it.amount" :kind="tone" class="shrink-0 font-medium" />
        </li>
      </ul>
      <p data-testid="rule-test-matches" class="text-meta text-muted-foreground">
        {{ t('emailRules.unparsedMatches', { n: result.unparsedMatches }) }}
      </p>
    </div>
  </section>
</template>
