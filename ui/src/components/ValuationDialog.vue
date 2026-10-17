<script setup lang="ts">
import { ref, useTemplateRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import AmountInput from '@/components/AmountInput.vue'
import DateInput from '@/components/DateInput.vue'
import MoneyText from '@/components/MoneyText.vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import * as ipc from '@/ipc'
import type { AccountDto, ValuationPointDto } from '@/ipc'
import { formatDateInput, todayIso } from '@/lib/date'
import { errorMessage, type Translate } from '@/lib/errors'
import { useAccountsStore } from '@/stores/accounts'
import { useSettingsStore } from '@/stores/settings'

// Nilai pasar akun investasi: yang dicatat backend adalah selisih terhadap saldo per tanggal itu.
const open = defineModel<boolean>('open', { required: true })
const props = defineProps<{ account: AccountDto }>()

const store = useAccountsStore()
const settings = useSettingsStore()
const { t } = useI18n()
const tr = t as unknown as Translate

const value = ref(0)
const date = ref(todayIso())
const history = ref<ValuationPointDto[]>([])
const error = ref<string | null>(null)
const busy = ref(false)

const dateInput = useTemplateRef<{ commit(): boolean }>('dateInput')
const valueInput = useTemplateRef<{ commit(): Promise<boolean> }>('valueInput')

async function loadHistory() {
  history.value = await ipc.valuationHistory(props.account.id)
}

watch(open, async (v) => {
  if (!v) return
  value.value = props.account.balance
  date.value = todayIso()
  error.value = null
  await loadHistory()
}, { immediate: true })

async function submit() {
  if (busy.value) return
  const ok = await Promise.all([dateInput.value?.commit() ?? true, valueInput.value?.commit() ?? true])
  if (!ok.every(Boolean) || error.value) return
  busy.value = true
  try {
    await store.recordValuation(props.account.id, date.value, value.value)
    await loadHistory()
  } catch (e) {
    error.value = errorMessage(e, tr)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="sm:max-w-md">
      <DialogHeader>
        <DialogTitle>{{ t('valuation.title', { name: account.name }) }}</DialogTitle>
        <DialogDescription>{{ t('valuation.hint') }}</DialogDescription>
      </DialogHeader>
      <form data-testid="valuation-form" class="flex flex-col gap-4" novalidate @submit.prevent="submit">
        <div class="grid grid-cols-2 gap-3">
          <div class="flex flex-col gap-2">
            <Label for="val-value">{{ t('valuation.value') }}</Label>
            <div class="relative">
              <span class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-muted-foreground">Rp</span>
              <AmountInput id="val-value" ref="valueInput" v-model="value" class="num pl-10 text-right" autofocus @error="error = $event" />
            </div>
          </div>
          <div class="flex flex-col gap-2">
            <Label for="val-date">{{ t('valuation.date') }}</Label>
            <DateInput id="val-date" ref="dateInput" v-model="date" />
          </div>
        </div>
        <p v-if="error" class="text-sm text-destructive" role="alert">{{ error }}</p>
        <DialogFooter>
          <Button type="button" variant="ghost" @click="open = false">{{ t('actions.close') }}</Button>
          <Button type="submit" :disabled="busy">{{ t('actions.save') }}</Button>
        </DialogFooter>
      </form>
      <section class="flex flex-col gap-2 border-t pt-4">
        <h3 class="text-sm font-medium">{{ t('valuation.history') }}</h3>
        <p v-if="history.length === 0" class="text-meta text-muted-foreground">{{ t('valuation.empty') }}</p>
        <ul v-else class="max-h-60 overflow-y-auto">
          <li
            v-for="p in history" :key="p.txnId" data-testid="valuation-row"
            class="grid h-9 grid-cols-[1fr_auto_7rem] items-center gap-3 text-sm"
          >
            <span class="text-muted-foreground">{{ formatDateInput(p.date, settings.locale) }}</span>
            <MoneyText :amount="p.value" />
            <span class="num text-right" :class="p.change > 0 ? 'text-income' : 'text-expense'">{{ p.change > 0 ? '+' : '' }}<MoneyText
              :amount="p.change" :currency="false"
            /></span>
          </li>
        </ul>
      </section>
    </DialogContent>
  </Dialog>
</template>
