<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import AmountInput from '@/components/AmountInput.vue'
import IconColorPicker from '@/components/IconColorPicker.vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import type { AccountDto, AccountFieldsDto, AccountKindDto } from '@/ipc'
import { ACCOUNT_KINDS, KIND_ICONS } from '@/lib/accounts'
import { todayIso } from '@/lib/date'
import { errorMessage, type Translate } from '@/lib/errors'
import { useAccountsStore } from '@/stores/accounts'

const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ account?: AccountDto; groupId?: string | null }>()
const emit = defineEmits<{ saved: [id: string] }>()

const store = useAccountsStore()
const { t } = useI18n()
const tr = t as unknown as Translate

// Select tidak menerima value kosong: pakai penanda untuk "Tanpa grup".
const NO_GROUP = '__none__'

const kind = ref<AccountKindDto>('bank')
const name = ref('')
const group = ref(NO_GROUP)
const opening = ref(0)
const openingDate = ref(todayIso())
const includeInNetWorth = ref(true)
const creditLimit = ref(0)
const statementDay = ref('')
const icon = ref<string | null>(null)
const color = ref<string | null>(null)
const error = ref<string | null>(null)
const amountError = ref<string | null>(null)
const busy = ref(false)

const editing = computed(() => props.account !== undefined)
const isCredit = computed(() => kind.value === 'credit_card')

function reset() {
  const a = props.account
  kind.value = a?.kind ?? 'bank'
  name.value = a?.name ?? ''
  group.value = (a ? a.groupId : props.groupId) ?? NO_GROUP
  opening.value = a?.openingBalance ?? 0
  openingDate.value = a?.openingDate ?? todayIso()
  includeInNetWorth.value = a?.includeInNetWorth ?? true
  creditLimit.value = a?.creditLimit ?? 0
  statementDay.value = a?.statementDay ? String(a.statementDay) : ''
  icon.value = a?.icon ?? null
  color.value = a?.color ?? null
  error.value = null
  amountError.value = null
}

watch(open, (v) => v && reset(), { immediate: true })

function fields(): AccountFieldsDto {
  return {
    name: name.value,
    groupId: group.value === NO_GROUP ? null : group.value,
    includeInNetWorth: includeInNetWorth.value,
    creditLimit: isCredit.value && creditLimit.value !== 0 ? creditLimit.value : null,
    statementDay: isCredit.value && statementDay.value !== '' ? Number(statementDay.value) : null,
    icon: icon.value,
    color: color.value,
    openingBalance: opening.value,
    openingDate: openingDate.value,
  }
}

async function submit() {
  if (busy.value || amountError.value) return
  // Cek cepat di UI; validasi sebenarnya tetap di Rust.
  if (name.value.trim() === '') {
    error.value = t('errors.name_empty')
    return
  }
  busy.value = true
  error.value = null
  try {
    const id = props.account
      ? (await store.updateAccount(props.account.id, fields()), props.account.id)
      : await store.createAccount(kind.value, fields())
    open.value = false
    emit('saved', id)
  } catch (e) {
    error.value = errorMessage(e, tr)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-lg">
      <DialogHeader>
        <DialogTitle>{{ editing ? t('accounts.edit') : t('accounts.new') }}</DialogTitle>
        <DialogDescription class="sr-only">{{ editing ? t('accounts.edit') : t('accounts.new') }}</DialogDescription>
      </DialogHeader>
      <form class="flex flex-col gap-4" novalidate @submit.prevent="submit" @keydown.ctrl.enter.prevent="submit">
        <div class="grid grid-cols-2 gap-3">
          <div class="col-span-2 flex flex-col gap-2">
            <Label for="acc-name">{{ t('accounts.name') }}</Label>
            <Input id="acc-name" v-model="name" maxlength="80" autofocus :aria-invalid="error ? true : undefined" />
          </div>
          <div class="flex flex-col gap-2">
            <Label id="acc-kind">{{ t('accounts.kind') }}</Label>
            <Select v-model="kind" :disabled="editing">
              <SelectTrigger data-testid="kind" aria-labelledby="acc-kind" class="w-full" :disabled="editing">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem v-for="k in ACCOUNT_KINDS" :key="k" :value="k">{{ t(`accountKinds.${k}`) }}</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="flex flex-col gap-2">
            <Label id="acc-group">{{ t('accounts.group') }}</Label>
            <Select v-model="group">
              <SelectTrigger aria-labelledby="acc-group" class="w-full"><SelectValue /></SelectTrigger>
              <SelectContent>
                <SelectItem :value="NO_GROUP">{{ t('accounts.noGroup') }}</SelectItem>
                <SelectItem v-for="g in store.overview.groups" :key="g.id" :value="g.id">{{ g.name }}</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div class="flex flex-col gap-2">
            <Label for="acc-opening">{{ t('accounts.openingBalance') }}</Label>
            <AmountInput id="acc-opening" v-model="opening" @error="amountError = $event" />
          </div>
          <div class="flex flex-col gap-2">
            <Label for="acc-opening-date">{{ t('accounts.openingDate') }}</Label>
            <Input id="acc-opening-date" v-model="openingDate" type="date" />
          </div>
          <template v-if="isCredit">
            <div class="flex flex-col gap-2" data-testid="credit-limit">
              <Label for="acc-limit">{{ t('accounts.creditLimit') }}</Label>
              <AmountInput id="acc-limit" v-model="creditLimit" @error="amountError = $event" />
            </div>
            <div class="flex flex-col gap-2" data-testid="statement-day">
              <Label for="acc-statement">{{ t('accounts.statementDay') }}</Label>
              <Input id="acc-statement" v-model="statementDay" type="number" min="1" max="31" />
            </div>
          </template>
        </div>
        <div class="flex items-center justify-between gap-3">
          <IconColorPicker v-model:icon="icon" v-model:color="color" :fallback-icon="KIND_ICONS[kind]" />
          <Label class="flex items-center gap-2 font-normal">
            <Switch v-model="includeInNetWorth" />
            {{ t('accounts.includeInNetWorth') }}
          </Label>
        </div>
        <p v-if="error || amountError" class="text-sm text-destructive" role="alert">{{ error ?? amountError }}</p>
        <DialogFooter>
          <Button type="button" variant="ghost" @click="open = false">{{ t('actions.cancel') }}</Button>
          <Button type="submit" :disabled="busy">
            {{ t('actions.save') }} <kbd class="ml-1 text-xs opacity-70">Ctrl+Enter</kbd>
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>
