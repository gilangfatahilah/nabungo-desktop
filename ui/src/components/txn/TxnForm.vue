<script setup lang="ts">
import { PlusIcon } from '@lucide/vue'
import { computed, nextTick, onMounted, reactive, ref, useTemplateRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import AmountInput from '@/components/AmountInput.vue'
import DateInput from '@/components/DateInput.vue'
import { Button } from '@/components/ui/button'
import { Kbd, KbdGroup } from '@/components/ui/kbd'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Textarea } from '@/components/ui/textarea'
import { payeeSuggestion } from '@/ipc'
import { errorMessage, type Translate } from '@/lib/errors'
import type { FormKind, TxnFormState } from '@/lib/txnForm'
import { useAccountsStore } from '@/stores/accounts'
import { useCategoriesStore } from '@/stores/categories'
import { useTransactionsStore } from '@/stores/transactions'
import CategoryCombobox from './CategoryCombobox.vue'
import PayeeCombobox from './PayeeCombobox.vue'
import SplitEditor from './SplitEditor.vue'
import TagInput from './TagInput.vue'

const props = defineProps<{ initial: TxnFormState; id: string | null }>()
const emit = defineEmits<{ saved: [id: string]; cancel: [] }>()

const { t } = useI18n()
const accounts = useAccountsStore()
const categories = useCategoriesStore()
const store = useTransactionsStore()

const s = reactive<TxnFormState>(JSON.parse(JSON.stringify(props.initial)) as TxnFormState)
const error = ref<string | null>(null)
const busy = ref(false)
// Saran dari payee hanya mengisi field yang belum disentuh user.
/** Fokus ke nominal saat form muncul (sheet mencegah auto-focus bawaannya). */
function focusAmount() {
  void nextTick(() => document.getElementById('txn-amount')?.focus())
}

// Tag & catatan terlipat kecuali sudah berisi (mode edit/duplikat), ui-guidelines §8.4.
const showMore = ref(s.tags.length > 0 || s.note.trim() !== '')

const touched = reactive({ category: s.categoryId !== null || s.split, account: props.id !== null })

type Committable = { commit(): boolean | Promise<boolean> }
const amountInput = useTemplateRef<Committable>('amountInput')
const feeInput = useTemplateRef<Committable>('feeInput')
const splitEditor = useTemplateRef<Committable>('splitEditor')
const dateInput = useTemplateRef<Committable>('dateInput')

const KINDS: FormKind[] = ['expense', 'income', 'transfer']
const transfer = computed(() => s.kind === 'transfer')
const activeAccounts = computed(() => accounts.overview.accounts.filter((a) => !a.archived || a.id === s.accountId || a.id === s.toAccountId))

watch(() => s.kind, () => {
  Object.assign(s, { categoryId: null, split: false, splits: [] })
  touched.category = false
})

onMounted(async () => {
  focusAmount()
  if (!categories.loaded) await categories.load()
  if (!accounts.loaded) await accounts.load()
})

function toggleSplit() {
  if (s.split) {
    Object.assign(s, { split: false, categoryId: s.splits[0]?.categoryId ?? null, splits: [] })
  } else {
    s.splits = [
      { categoryId: s.categoryId, amount: s.amount, memo: '' },
      { categoryId: null, amount: 0, memo: '' },
    ]
    s.split = true
    s.categoryId = null
  }
  touched.category = true
}

async function onPayee(name: string) {
  if (!name.trim() || (touched.category && touched.account)) return
  const sug = await payeeSuggestion(name)
  if (!sug) return
  const cat = categories.list.find((c) => c.id === sug.categoryId)
  if (!touched.category && !transfer.value && cat && !cat.archived && cat.kind === s.kind) s.categoryId = cat.id
  const acc = accounts.overview.accounts.find((a) => a.id === sug.accountId)
  if (!touched.account && acc && !acc.archived) s.accountId = acc.id
}

async function submit() {
  if (busy.value) return
  const refs = [dateInput.value, amountInput.value, transfer.value && s.feeEnabled ? feeInput.value : null, s.split && !transfer.value ? splitEditor.value : null]
  const ok = await Promise.all(refs.map((r) => r?.commit() ?? true))
  if (!ok.every(Boolean)) return
  busy.value = true
  error.value = null
  try {
    emit('saved', await store.save(props.id, s))
  } catch (e) {
    error.value = errorMessage(e, t as unknown as Translate)
  } finally {
    busy.value = false
  }
}

</script>

<template>
  <form class="flex min-h-0 flex-1 flex-col" novalidate @submit.prevent="submit" @keydown.ctrl.enter.prevent="submit" @keydown.meta.enter.prevent="submit">
    <div data-testid="txn-form-body" class="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-4 pt-1 pb-4">
      <Tabs v-model="s.kind">
        <TabsList class="w-full">
          <TabsTrigger v-for="k in KINDS" :key="k" :value="k" :data-testid="`tab-${k}`">{{ t(`txn.kinds.${k}`) }}</TabsTrigger>
        </TabsList>
      </Tabs>

      <div class="flex flex-col gap-2">
        <Label for="txn-amount">{{ t('txn.amount') }}</Label>
        <div class="relative">
          <span data-testid="txn-amount-prefix" class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-muted-foreground">Rp</span>
          <AmountInput id="txn-amount" ref="amountInput" v-model="s.amount" class="num h-10 pl-10 text-right text-xl font-semibold md:text-xl" autofocus @error="error = $event" />
        </div>
        <p v-if="s.kind === 'expense' && s.amount < 0" class="text-meta text-muted-foreground">{{ t('txn.refundHint') }}</p>
      </div>

      <!-- Tombol split tampil di baris label, tapi di DOM sesudah field kategori agar Tab dari nominal langsung ke kategori. -->
      <div v-if="!transfer" class="grid grid-cols-[1fr_auto] items-center gap-2">
        <Label class="col-start-1 row-start-1" :for="s.split ? undefined : 'txn-category'">{{ t('txn.category') }}</Label>
        <div class="col-span-2 row-start-2">
          <SplitEditor v-if="s.split" ref="splitEditor" v-model="s.splits" :kind="s.kind as 'income' | 'expense'" :total="s.amount" @error="error = $event" />
          <CategoryCombobox v-else id="txn-category" v-model="s.categoryId" :kind="s.kind as 'income' | 'expense'" @picked="touched.category = true" />
        </div>
        <Button type="button" variant="ghost" size="xs" class="col-start-2 row-start-1 text-muted-foreground" data-testid="split-toggle" @click="toggleSplit">
          {{ s.split ? t('txn.unsplit') : t('txn.split') }}
        </Button>
      </div>

      <div class="grid grid-cols-2 gap-3">
        <div class="flex flex-col gap-2">
          <Label id="txn-account-label">{{ transfer ? t('txn.from') : t('txn.account') }}</Label>
          <Select v-model="s.accountId" @update:model-value="touched.account = true">
            <SelectTrigger aria-labelledby="txn-account-label" class="w-full" data-testid="txn-account"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem v-for="a in activeAccounts" :key="a.id" :value="a.id">{{ a.name }}</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div v-if="transfer" class="flex flex-col gap-2">
          <Label id="txn-to-label">{{ t('txn.to') }}</Label>
          <Select v-model="s.toAccountId">
            <SelectTrigger aria-labelledby="txn-to-label" class="w-full" data-testid="txn-to-account"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem v-for="a in activeAccounts" :key="a.id" :value="a.id" :disabled="a.id === s.accountId">{{ a.name }}</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div class="flex flex-col gap-2">
          <Label for="txn-date">{{ t('txn.date') }}</Label>
          <DateInput id="txn-date" ref="dateInput" v-model="s.date" />
        </div>
      </div>

      <div v-if="transfer" class="flex flex-col gap-2">
        <Label class="flex items-center gap-2 font-normal">
          <Switch v-model="s.feeEnabled" data-testid="fee-toggle" />
          {{ t('txn.fee') }}
        </Label>
        <div v-if="s.feeEnabled" class="grid grid-cols-[8rem_1fr] gap-2">
          <AmountInput ref="feeInput" v-model="s.fee" class="num text-right" :aria-label="t('txn.feeAmount')" @error="error = $event" />
          <CategoryCombobox id="txn-fee-category" v-model="s.feeCategoryId" kind="expense" :aria-label="t('txn.feeCategory')" />
        </div>
      </div>

      <div class="flex flex-col gap-2">
        <Label for="txn-payee">{{ t('txn.payee') }}</Label>
        <PayeeCombobox id="txn-payee" v-model="s.payee" @picked="onPayee" />
      </div>

      <template v-if="showMore">
        <div class="flex flex-col gap-2">
          <Label for="txn-tags">{{ t('txn.tags') }}</Label>
          <TagInput id="txn-tags" v-model="s.tags" :placeholder="t('txn.tagsHint')" />
        </div>
        <div class="flex flex-col gap-2">
          <Label for="txn-note">{{ t('txn.note') }}</Label>
          <Textarea id="txn-note" v-model="s.note" rows="2" />
        </div>
      </template>
      <Button v-else type="button" variant="ghost" size="sm" class="self-start text-muted-foreground" data-testid="txn-more" @click="showMore = true">
        <PlusIcon /> {{ t('txn.addDetails') }}
      </Button>

      <p v-if="error" class="text-meta text-destructive" role="alert">{{ error }}</p>
    </div>

    <div data-testid="txn-form-footer" class="flex justify-end gap-2 border-t px-4 py-3">
      <Button type="button" variant="ghost" @click="emit('cancel')">{{ t('actions.cancel') }}</Button>
      <Button type="submit" :disabled="busy">
        {{ t('txn.save') }}
        <KbdGroup class="ml-1"><Kbd>Ctrl</Kbd><Kbd>Enter</Kbd></KbdGroup>
      </Button>
    </div>
  </form>
</template>
